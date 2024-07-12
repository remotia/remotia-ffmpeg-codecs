use std::{ffi::CString, ptr::NonNull, sync::Arc};

use rsmpeg::avcodec::{AVCodec, AVCodecContext};

use tokio::sync::Mutex;

use crate::{builder::unwrap_mandatory, ffi, scaling::Scaler};

use super::options::Options;

mod puller;
mod pusher;
pub mod fillers;

pub use puller::*;
pub use pusher::*;

pub struct EncoderBuilder<T, K: Copy, EFE: Copy, P: Copy> {
    codec_id: Option<String>,

    filler: Option<T>,
    encoded_buffer_key: Option<K>,

    options: Option<Options>,

    scaler: Option<Scaler>,

    encoder_flushed_error: Option<EFE>,

    frame_id_prop: Option<P>
}

impl<T, K: Copy, EFE: Copy, P: Copy> EncoderBuilder<T, K, EFE, P> {
    pub fn new() -> Self {
        Self {
            codec_id: None,
            filler: None,
            encoded_buffer_key: None,
            options: None,
            scaler: None,
            encoder_flushed_error: None,
            frame_id_prop: None
        }
    }

    builder_set!(filler, T);
    builder_set!(encoded_buffer_key, K);
    builder_set!(options, Options);
    builder_set!(scaler, Scaler);
    builder_set!(encoder_flushed_error, EFE);
    builder_set!(frame_id_prop, P);

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }

    pub fn build(self) -> (EncoderPusher<T, P>, EncoderPuller<K, EFE>) {
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
        let encoded_buffer_key = unwrap_mandatory(self.encoded_buffer_key);
        let encoder_flushed_error = unwrap_mandatory(self.encoder_flushed_error);
        let frame_id_prop = unwrap_mandatory(self.frame_id_prop);

        (
            EncoderPusher {
                encode_context: encode_context.clone(),
                scaler,
                filler,
                frame_id_prop
            },
            EncoderPuller {
                encode_context: encode_context.clone(),
                encoded_buffer_key,
                encoder_flushed_error
            },
        )
    }
}
