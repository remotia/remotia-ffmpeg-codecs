use std::{ffi::CString, ptr::NonNull, sync::Arc};

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    swscale::SwsContext,
};

use tokio::sync::Mutex;

use crate::ffi;

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
        let codec_id = self.codec_id.expect("Missing mandatory field 'codec_id'");
        let width = self.width.expect("Missing mandatory field 'width'");
        let height = self.height.expect("Missing mandatory field 'height'");
        let options = self.options.unwrap_or_default();

        let input_pixel_format = self
            .input_pixel_format
            .expect("Missing mandatory field 'input_pixel_format'");
        let codec_pixel_format = self
            .codec_pixel_format
            .expect("Missing mandatory field 'codec_pixel_format'");

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

        let rgba_buffer_key = self
            .rgba_buffer_key
            .expect("Missing mandatory field 'rgba_buffer_key'");
        let encoded_buffer_key = self
            .encoded_buffer_key
            .expect("Missing mandatory field 'encoded_buffer_key'");

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

    pub fn width(mut self, width: i32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: i32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn rgba_buffer_key(mut self, rgba_buffer_key: K) -> Self {
        self.rgba_buffer_key = Some(rgba_buffer_key);
        self
    }

    pub fn encoded_buffer_key(mut self, encoded_buffer_key: K) -> Self {
        self.encoded_buffer_key = Some(encoded_buffer_key);
        self
    }

    pub fn options(mut self, options: Options) -> Self {
        self.options = Some(options);
        self
    }

    pub fn input_pixel_format(mut self, input_pixel_format: ffi::AVPixelFormat) -> Self {
        self.input_pixel_format = Some(input_pixel_format);
        self
    }

    pub fn codec_pixel_format(mut self, codec_pixel_format: ffi::AVPixelFormat) -> Self {
        self.codec_pixel_format = Some(codec_pixel_format);
        self
    }

    pub fn scaling_flags(mut self, scaling_flags: u32) -> Self {
        self.scaling_flags = Some(scaling_flags);
        self
    }

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }
}
