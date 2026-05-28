use std::{ffi::CString, sync::Arc};

use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParserContext};

use remotia::pipeline::PipelineHandle;
use tokio::sync::{mpsc, Mutex};

use crate::{builder::unwrap_mandatory, options::Options, scaling::Scaler};

mod puller;
mod pusher;

pub use puller::*;
pub use pusher::*;

pub struct DecoderBuilder {
    codec_id: Option<String>,
    options: Option<Options>,
    scaler: Option<Scaler>,
    pipeline_handle: Option<PipelineHandle>,
}

impl Default for DecoderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoderBuilder {
    pub fn new() -> Self {
        Self {
            codec_id: None,
            options: None,
            scaler: None,
            pipeline_handle: None,
        }
    }

    builder_set!(options, Options);
    builder_set!(scaler, Scaler);

    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }

    pub fn pipeline_handle(mut self, handle: PipelineHandle) -> Self {
        self.pipeline_handle = Some(handle);
        self
    }

    pub fn build(self) -> (DecoderPusher, DecoderPuller) {
        let codec_id = unwrap_mandatory(self.codec_id);
        let options = self.options.unwrap_or_default().to_av_dict();

        let codec_id_string = CString::new(codec_id).unwrap();
        let decoder = AVCodec::find_decoder_by_name(&codec_id_string).unwrap();
        let parser_context = AVCodecParserContext::init(decoder.id).unwrap();

        let decode_context = {
            let mut decode_context = AVCodecContext::new(&decoder);
            decode_context.open(Some(options)).unwrap();

            Arc::new(Mutex::new(decode_context))
        };

        let scaler = unwrap_mandatory(self.scaler);

        let (frame_tx, frame_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        (
            DecoderPusher {
                decode_context: decode_context.clone(),
                parser_context,
                scaler,
                frame_tx: Some(frame_tx),
                pipeline_handle: self.pipeline_handle.clone(),
                eof_processed: false,
            },
            DecoderPuller {
                _decode_context: decode_context.clone(),
                frame_rx,
                pipeline_handle: self.pipeline_handle,
            },
        )
    }
}
