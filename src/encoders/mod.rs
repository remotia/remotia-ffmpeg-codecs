use std::{ffi::CString, ptr::NonNull, sync::Arc};

use rsmpeg::avcodec::{AVCodec, AVCodecContext};

use tokio::sync::Mutex;

use crate::{builder::unwrap_mandatory, ffi, scaling::Scaler};

use super::options::Options;

pub mod fillers;
mod puller;
mod pusher;

pub use puller::*;
pub use pusher::*;

pub struct EncoderBuilder<T> {
    codec_id: Option<String>,
    filler: Option<T>,
    options: Option<Options>,
    scaler: Option<Scaler>,
}

impl<T> Default for EncoderBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> EncoderBuilder<T> {
    pub fn new() -> Self {
        Self {
            codec_id: None,
            filler: None,
            options: None,
            scaler: None,
        }
    }

    builder_set!(filler, T);
    builder_set!(options, Options);
    builder_set!(scaler, Scaler);

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }

    pub fn build(self) -> (EncoderPusher<T>, EncoderPuller) {
        let codec_id = unwrap_mandatory(self.codec_id);
        let options = self.options.unwrap_or_default();

        let scaler = unwrap_mandatory(self.scaler);

        let encode_context = {
            let codec_id_string = CString::new(codec_id).unwrap();
            let encoder = AVCodec::find_encoder_by_name(&codec_id_string).unwrap();
            let mut encode_context = AVCodecContext::new(&encoder);
            encode_context.set_width(scaler.scaled_frame().width);
            encode_context.set_height(scaler.scaled_frame().height);
            encode_context.set_pix_fmt(scaler.scaled_frame().format);
            encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 * 1000 });
            encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
            let mut encode_context = unsafe {
                let raw_encode_context = encode_context.into_raw().as_ptr();
                AVCodecContext::from_raw(NonNull::new(raw_encode_context).unwrap())
            };

            let options_dict = options.to_av_dict();

            encode_context.open(Some(options_dict)).unwrap();

            Arc::new(Mutex::new(encode_context))
        };

        let filler = unwrap_mandatory(self.filler);

        (
            EncoderPusher {
                encode_context: encode_context.clone(),
                scaler,
                filler,
            },
            EncoderPuller {
                encode_context: encode_context.clone(),
            },
        )
    }
}
