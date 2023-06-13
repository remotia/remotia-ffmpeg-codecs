use std::{sync::Arc, ffi::CString};

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext},
    swscale::SwsContext,
};

use tokio::sync::Mutex;

use crate::{encoders::options::Options, ffi};

mod pusher;
mod puller;

pub use pusher::*;
pub use puller::*;

pub struct DecoderBuilder<K, E> {
    codec_id: Option<String>,

    encoded_buffer_key: Option<K>,
    decoded_buffer_key: Option<K>,

    drain_error: Option<E>,
    codec_error: Option<E>,

    options: Option<Options>,

    width: Option<i32>,
    height: Option<i32>,
    input_pixel_format: Option<ffi::AVPixelFormat>,
    output_pixel_format: Option<ffi::AVPixelFormat>,
    scaling_flags: Option<u32>,
}

// TODO: Fix all those unsafe impl
unsafe impl<K, E> Send for DecoderBuilder<K, E> {}

impl<K, E> DecoderBuilder<K, E> {
    pub fn new() -> Self {
        Self {
            codec_id: None,
            encoded_buffer_key: None,
            decoded_buffer_key: None,
            drain_error: None,
            codec_error: None,
            width: None,
            height: None,
            input_pixel_format: None,
            output_pixel_format: None,
            scaling_flags: None,
            options: None,
        }
    }

    pub fn build(self) -> (DecoderPusher<K, E>, DecoderPuller<K, E>) {
        let codec_id = self.codec_id.expect("Missing mandatory field 'codec_id'");
        let options = self.options.unwrap_or_default().to_av_dict();

        let codec_id_string = CString::new(codec_id).unwrap();
        let decoder = AVCodec::find_decoder_by_name(&codec_id_string).unwrap();
        let decode_context = {
            let mut decode_context = AVCodecContext::new(&decoder);
            decode_context.open(Some(options)).unwrap();

            Arc::new(Mutex::new(decode_context))
        };

        let width = self.width.expect("Missing mandatory field 'width'");
        let height = self.height.expect("Missing mandatory field 'height'");
        let input_pixel_format = self
            .input_pixel_format
            .expect("Missing mandatory field 'input_pixel_format'");
        let output_pixel_format = self
            .output_pixel_format
            .expect("Missing mandatory field 'output_pixel_format'");

        let scaling_flags = self.scaling_flags.unwrap_or(ffi::SWS_BILINEAR);

        let scaling_context = {
            SwsContext::get_context(
                width,
                height,
                input_pixel_format,
                width,
                height,
                output_pixel_format,
                scaling_flags,
            )
            .unwrap()
        };

        let parser_context = AVCodecParserContext::find(decoder.id).unwrap();

        let encoded_buffer_key = self
            .encoded_buffer_key
            .expect("Missing mandantory field 'encoded_buffer_key'");
        let codec_error = self
            .codec_error
            .expect("Missing mandatory field 'codec_error'");
        let decoded_buffer_key = self
            .decoded_buffer_key
            .expect("Missing mandantory field 'decoded_buffer_key'");
        let drain_error = self
            .drain_error
            .expect("Missing mandatory field 'drain_error'");

        (
            DecoderPusher {
                decode_context: decode_context.clone(),
                parser_context,
                encoded_buffer_key,
                codec_error,
            },
            DecoderPuller {
                decode_context: decode_context.clone(),
                scaling_context,
                decoded_buffer_key,
                drain_error,
            },
        )
    }

    pub fn encoded_buffer_key(mut self, encoded_buffer_key: K) -> Self {
        self.encoded_buffer_key = Some(encoded_buffer_key);
        self
    }

    pub fn decoded_buffer_key(mut self, decoded_buffer_key: K) -> Self {
        self.decoded_buffer_key = Some(decoded_buffer_key);
        self
    }

    pub fn drain_error(mut self, drain_error: E) -> Self {
        self.drain_error = Some(drain_error);
        self
    }

    pub fn codec_error(mut self, codec_error: E) -> Self {
        self.codec_error = Some(codec_error);
        self
    }

    pub fn options(mut self, options: Options) -> Self {
        self.options = Some(options);
        self
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: i32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn input_pixel_format(mut self, input_pixel_format: ffi::AVPixelFormat) -> Self {
        self.input_pixel_format = Some(input_pixel_format);
        self
    }

    pub fn output_pixel_format(mut self, output_pixel_format: ffi::AVPixelFormat) -> Self {
        self.output_pixel_format = Some(output_pixel_format);
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


