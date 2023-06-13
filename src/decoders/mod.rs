use std::{sync::Arc, ffi::CString};

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext},
    swscale::SwsContext, avutil::AVFrame,
};

use tokio::sync::Mutex;

use crate::{options::Options, ffi, builder::unwrap_mandatory};

mod utils;

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
    codec_pixel_format: Option<ffi::AVPixelFormat>,
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
            codec_pixel_format: None,
            output_pixel_format: None,
            scaling_flags: None,
            options: None,
        }
    }

    pub fn build(self) -> (DecoderPusher<K, E>, DecoderPuller<K, E>) {
        let codec_id = unwrap_mandatory(self.codec_id);
        let options = self.options.unwrap_or_default().to_av_dict();

        let codec_id_string = CString::new(codec_id).unwrap();
        let decoder = AVCodec::find_decoder_by_name(&codec_id_string).unwrap();
        let decode_context = {
            let mut decode_context = AVCodecContext::new(&decoder);
            decode_context.open(Some(options)).unwrap();

            Arc::new(Mutex::new(decode_context))
        };

        let width = unwrap_mandatory(self.width);
        let height = unwrap_mandatory(self.height);
        let codec_pixel_format = unwrap_mandatory(self.codec_pixel_format);
        let output_pixel_format = unwrap_mandatory(self.output_pixel_format);

        let scaling_flags = self.scaling_flags.unwrap_or(ffi::SWS_BILINEAR);

        let scaling_context = {
            SwsContext::get_context(
                width,
                height,
                codec_pixel_format,
                width,
                height,
                output_pixel_format,
                scaling_flags,
            )
            .unwrap()
        };

        let parser_context = AVCodecParserContext::find(decoder.id).unwrap();

        let encoded_buffer_key = unwrap_mandatory(self.encoded_buffer_key);
        let codec_error = unwrap_mandatory(self.codec_error);
        let decoded_buffer_key = unwrap_mandatory(self.decoded_buffer_key);
        let drain_error = unwrap_mandatory(self.drain_error);

        let output_avframe = {
            let mut avframe = AVFrame::new();
            avframe.set_format(output_pixel_format);
            avframe.set_width(width);
            avframe.set_height(height);
            avframe.alloc_buffer().unwrap();
            avframe
        };

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
                output_avframe
            },
        )
    }

    builder_set!(encoded_buffer_key, K);
    builder_set!(decoded_buffer_key, K);
    builder_set!(drain_error, E);
    builder_set!(codec_error, E);
    builder_set!(options, Options);
    builder_set!(width, i32);
    builder_set!(height, i32);
    builder_set!(codec_pixel_format, ffi::AVPixelFormat);
    builder_set!(output_pixel_format, ffi::AVPixelFormat);
    builder_set!(scaling_flags, u32);

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }
}


