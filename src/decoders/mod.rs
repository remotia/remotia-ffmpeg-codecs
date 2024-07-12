use std::{ffi::CString, sync::Arc};

use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket};

use tokio::sync::Mutex;

use crate::{builder::unwrap_mandatory, options::Options, scaling::Scaler};

mod utils;

mod puller;
mod pusher;

pub use puller::*;
pub use pusher::*;

pub struct DecoderBuilder<K, E, P> {
    codec_id: Option<String>,

    encoded_buffer_key: Option<K>,
    decoded_buffer_key: Option<K>,

    drain_error: Option<E>,
    codec_error: Option<E>,

    frame_id_prop: Option<P>,

    options: Option<Options>,

    scaler: Option<Scaler>,
}

// TODO: Fix all those unsafe impl
unsafe impl<K, E, P> Send for DecoderBuilder<K, E, P> {}

impl<K, E, P: Copy> DecoderBuilder<K, E, P> {
    pub fn new() -> Self {
        Self {
            codec_id: None,
            encoded_buffer_key: None,
            decoded_buffer_key: None,
            drain_error: None,
            codec_error: None,
            frame_id_prop: None,
            options: None,
            scaler: None,
        }
    }

    builder_set!(encoded_buffer_key, K);
    builder_set!(decoded_buffer_key, K);
    builder_set!(drain_error, E);
    builder_set!(codec_error, E);
    builder_set!(frame_id_prop, P);
    builder_set!(options, Options);
    builder_set!(scaler, Scaler);

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }

    pub fn build(self) -> (DecoderPusher<K, E, P>, DecoderPuller<K, E, P>) {
        let codec_id = unwrap_mandatory(self.codec_id);
        let options = self.options.unwrap_or_default().to_av_dict();

        let codec_id_string = CString::new(codec_id).unwrap();
        let decoder = AVCodec::find_decoder_by_name(&codec_id_string).unwrap();
        let parser_context = AVCodecParserContext::find(decoder.id).unwrap();

        let decode_context = {
            let mut decode_context = AVCodecContext::new(&decoder);
            decode_context.open(Some(options)).unwrap();

            Arc::new(Mutex::new(decode_context))
        };

        let scaler = unwrap_mandatory(self.scaler);

        let encoded_buffer_key = unwrap_mandatory(self.encoded_buffer_key);
        let codec_error = unwrap_mandatory(self.codec_error);
        let decoded_buffer_key = unwrap_mandatory(self.decoded_buffer_key);
        let drain_error = unwrap_mandatory(self.drain_error);
        let frame_id_prop = unwrap_mandatory(self.frame_id_prop);

        (
            DecoderPusher {
                decode_context: decode_context.clone(),
                parser_context,
                encoded_buffer_key,
                codec_error,
                frame_id_prop,
            },
            DecoderPuller {
                decode_context: decode_context.clone(),
                scaler,
                decoded_buffer_key,
                drain_error,
                frame_id_prop,
            },
        )
    }
}
