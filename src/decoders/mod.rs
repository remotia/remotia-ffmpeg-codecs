use std::{ffi::CString, sync::Arc};

use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket};

use tokio::sync::Mutex;

use crate::{builder::unwrap_mandatory, options::Options, scaling::Scaler};

mod utils;

mod puller;
mod pusher;

pub use puller::*;
pub use pusher::*;

pub struct DecoderBuilder {
    codec_id: Option<String>,
    options: Option<Options>,
    scaler: Option<Scaler>,
}

impl DecoderBuilder {
    pub fn new() -> Self {
        Self {
            codec_id: None,
            options: None,
            scaler: None,
        }
    }

    builder_set!(options, Options);
    builder_set!(scaler, Scaler);

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }

    pub fn build(self) -> (DecoderPusher, DecoderPuller) {
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

        (
            DecoderPusher {
                decode_context: decode_context.clone(),
                parser_context,
            },
            DecoderPuller {
                decode_context: decode_context.clone(),
                scaler,
            },
        )
    }
}
