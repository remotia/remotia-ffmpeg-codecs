use log::{debug};
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext},
    avutil::AVDictionary
};

use cstr::cstr;

use remotia::{traits::FrameProcessor, types::FrameData};

use async_trait::async_trait;

use super::utils::{decode_to_yuv};

pub struct H264Decoder {
    decode_context: AVCodecContext,
    parser_context: AVCodecParserContext,
}

// TODO: Fix all those unsafe impl
unsafe impl Send for H264Decoder {}

impl H264Decoder {
    pub fn new() -> Self {
        let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).unwrap();

        let options = AVDictionary::new(cstr!(""), cstr!(""), 0)
            .set(cstr!("threads"), cstr!("4"), 0)
            .set(cstr!("thread_type"), cstr!("slice"), 0);

        H264Decoder {
            decode_context: {
                let mut decode_context = AVCodecContext::new(&decoder);
                decode_context.open(Some(options)).unwrap();

                decode_context
            },

            parser_context: AVCodecParserContext::find(decoder.id).unwrap(),
        }
    }
}

impl Default for H264Decoder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameProcessor for H264Decoder {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let decode_result = decode_to_yuv(
            &mut self.decode_context,
            &mut self.parser_context,
            &mut frame_data,
        );

        if let Err(drop_reason) = decode_result {
            debug!("Dropping frame, reason: {:?}", drop_reason);
            frame_data.set_drop_reason(Some(drop_reason));
        }

        Some(frame_data)
    }
}
