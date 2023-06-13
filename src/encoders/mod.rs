use std::{ffi::CString, ptr::NonNull, sync::Arc};

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    swscale::SwsContext,
};

use tokio::sync::Mutex;

use crate::{builder::unwrap_mandatory, ffi};

use super::options::Options;

mod puller;
mod pusher;

pub use puller::*;
pub use pusher::*;

pub struct EncoderBuilder<K: Copy> {
    codec_id: Option<String>,

    width: Option<i32>,
    height: Option<i32>,

    rgba_buffer_key: Option<K>,
    encoded_buffer_key: Option<K>,

    options: Option<Options>,

    input_pixel_format: Option<ffi::AVPixelFormat>,
    codec_pixel_format: Option<ffi::AVPixelFormat>,
    scaling_flags: Option<u32>,
}

impl<K: Copy> EncoderBuilder<K> {
    pub fn new() -> Self {
        Self {
            codec_id: None,
            width: None,
            height: None,
            rgba_buffer_key: None,
            encoded_buffer_key: None,
            options: None,
            input_pixel_format: None,
            codec_pixel_format: None,
            scaling_flags: None,
        }
    }

    pub fn build(self) -> (EncoderPusher<K>, EncoderPuller<K>) {
        let codec_id = unwrap_mandatory(self.codec_id);
        let width = unwrap_mandatory(self.width);
        let height = unwrap_mandatory(self.height);
        let options = self.options.unwrap_or_default();

        let input_pixel_format = unwrap_mandatory(self.input_pixel_format);
        let codec_pixel_format = unwrap_mandatory(self.codec_pixel_format);

        let encode_context = {
            let codec_id_string = CString::new(codec_id).unwrap();
            let encoder = AVCodec::find_encoder_by_name(&codec_id_string).unwrap();
            let mut encode_context = AVCodecContext::new(&encoder);
            encode_context.set_width(width);
            encode_context.set_height(height);
            encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 * 1000 });
            encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
            encode_context.set_pix_fmt(codec_pixel_format);
            let mut encode_context = unsafe {
                let raw_encode_context = encode_context.into_raw().as_ptr();
                AVCodecContext::from_raw(NonNull::new(raw_encode_context).unwrap())
            };

            let options_dict = options.to_av_dict();

            encode_context.open(Some(options_dict)).unwrap();

            Arc::new(Mutex::new(encode_context))
        };

        let scaling_flags = self.scaling_flags.unwrap_or(ffi::SWS_BILINEAR);

        let scaling_context = SwsContext::get_context(
            width,
            height,
            input_pixel_format,
            width,
            height,
            codec_pixel_format,
            scaling_flags,
        )
        .unwrap();

        let rgba_buffer_key = unwrap_mandatory(self.rgba_buffer_key);
        let encoded_buffer_key = unwrap_mandatory(self.encoded_buffer_key);

        (
            EncoderPusher {
                encode_context: encode_context.clone(),
                scaling_context,
                rgba_buffer_key,
            },
            EncoderPuller {
                encode_context: encode_context.clone(),
                encoded_buffer_key,
            },
        )
    }

    builder_set!(width, i32);
    builder_set!(height, i32);
    builder_set!(rgba_buffer_key, K);
    builder_set!(encoded_buffer_key, K);
    builder_set!(options, Options);
    builder_set!(input_pixel_format, ffi::AVPixelFormat);
    builder_set!(codec_pixel_format, ffi::AVPixelFormat);
    builder_set!(scaling_flags, u32);

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }
}
