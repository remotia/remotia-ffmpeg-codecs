use std::{sync::Arc, time::Duration};

use log::debug;
use rsmpeg::{
    avcodec::{AVCodecContext, AVCodecParserContext},
    error::RsmpegError,
};

use remotia::traits::FrameProcessor;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::FFMpegCodec;

use super::utils::parse_and_send_packets;

pub struct DecoderPusher {
    pub(super) parser_context: AVCodecParserContext,
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
}

#[async_trait]
impl<F> FrameProcessor<F> for DecoderPusher
where
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let frame_id = frame_data.get_frame_id();

        let encoded_packets_buffer = frame_data.get_packet_data_buffer();
        // let encoded_packets_buffer = &encoded_buffer[..encoded_buffer.len()];

        loop {
            let mut decode_context = self.decode_context.lock().await;

            match parse_and_send_packets(
                &mut decode_context,
                &mut self.parser_context,
                encoded_packets_buffer,
                frame_id,
            ) {
                Ok(_) => {
                    debug!("Packet pushed, breaking the loop...");
                    break;
                }
                Err(RsmpegError::DecoderFullError) => {
                    debug!("Decoder full, trying to push the packet again...");
                    drop(decode_context);
                    tokio::task::yield_now().await;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(error) => {
                    debug!("Unable to send packet to decode, reason: {:?}", error);
                    frame_data.report_codec_error(error);
                    break;
                }
            }
        }

        Some(frame_data)
    }
}
