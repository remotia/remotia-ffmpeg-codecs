use bytes::BytesMut;
use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext},
    avutil::AVDictionary,
};

use cstr::cstr;

use remotia::traits::{FrameProcessor, PullableFrameProperties};

use async_trait::async_trait;

use super::utils::decode_to_yuv;

pub struct H264Decoder<K> {
    decode_context: AVCodecContext,
    parser_context: AVCodecParserContext,

    encoded_buffer_key: K,
    y_buffer_key: K,
    cb_buffer_key: K,
    cr_buffer_key: K
}

// TODO: Fix all those unsafe impl
unsafe impl<K> Send for H264Decoder<K> {}

impl<K> H264Decoder<K> {
    pub fn new(
        encoded_buffer_key: K,
        y_buffer_key: K,
        cb_buffer_key: K,
        cr_buffer_key: K
    ) -> Self {
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
            encoded_buffer_key,
            y_buffer_key,
            cb_buffer_key,
            cr_buffer_key
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for H264Decoder<K>
where
    K: Send + Copy,
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let timestamp = 0 as i64; // TODO: Extract timestamp from properties

        let encoded_buffer = frame_data.pull(&self.encoded_buffer_key).unwrap();
        let mut y_buffer = frame_data.pull(&self.y_buffer_key).unwrap();
        let mut cb_buffer = frame_data.pull(&self.cb_buffer_key).unwrap();
        let mut cr_buffer = frame_data.pull(&self.cr_buffer_key).unwrap();

        let encoded_packets_buffer = &encoded_buffer[..encoded_buffer.len()];

        let decode_result = decode_to_yuv(
            &mut self.decode_context,
            &mut self.parser_context,
            timestamp,
            encoded_packets_buffer,
            &mut y_buffer,
            &mut cb_buffer,
            &mut cr_buffer
        );

        if let Err(drop_reason) = decode_result {
            debug!("Dropping frame, reason: {:?}", drop_reason);
            // TODO: Add error report
        }

        frame_data.push(self.encoded_buffer_key, encoded_buffer);
        frame_data.push(self.y_buffer_key, y_buffer);
        frame_data.push(self.cb_buffer_key, cb_buffer);
        frame_data.push(self.cr_buffer_key, cr_buffer);

        Some(frame_data)
    }
}
