use std::sync::Arc;

use log::debug;
use rsmpeg::{
    avcodec::{AVCodecContext, AVCodecParserContext},
};

use remotia::{
    buffers::BytesMut,
    traits::{BorrowMutFrameProperties, FrameError, FrameProcessor},
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::utils::parse_and_send_packets;

pub struct DecoderPusher<K, E> {
    pub(super) parser_context: AVCodecParserContext,
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,

    pub(super) encoded_buffer_key: K,

    pub(super) codec_error: E,
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for DecoderPusher<K, E>
where
    K: Send + Copy,
    E: Send + Copy,
    F: BorrowMutFrameProperties<K, BytesMut> + FrameError<E> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let timestamp = 0 as i64; // TODO: Extract timestamp from properties

        let encoded_buffer = frame_data.get_mut_ref(&self.encoded_buffer_key).unwrap();

        let encoded_packets_buffer = &encoded_buffer[..encoded_buffer.len()];

        let mut decode_context = self.decode_context.lock().await;

        let send_result = parse_and_send_packets(
            &mut decode_context,
            &mut self.parser_context,
            encoded_packets_buffer,
            timestamp,
        );

        if let Err(error) = send_result {
            debug!("Dropping frame, reason: {:?}", error);
            frame_data.report_error(self.codec_error);
            return Some(frame_data);
        }

        Some(frame_data)
    }
}