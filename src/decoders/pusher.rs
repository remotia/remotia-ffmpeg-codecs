use std::sync::Arc;

use rsmpeg::avcodec::{AVCodecContext, AVCodecParserContext};
use rsmpeg::error::RsmpegError;

use remotia::traits::FrameProcessor;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::FFMpegCodec;

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
        if frame_data.is_eof() {
            log::debug!("DecoderPusher: EOF, flushing decoder");
            let mut decode_context = self.decode_context.lock().await;
            let _ = decode_context.send_packet(None);
            return Some(frame_data);
        }

        let frame_id = frame_data.get_frame_id();

        let encoded_packets_buffer = frame_data.get_packet_data_buffer();

        let mut decode_context = self.decode_context.lock().await;

        let send_result = parse_and_send_packets(
            &mut decode_context,
            &mut self.parser_context,
            encoded_packets_buffer,
            frame_id,
        );

        if let Err(()) = send_result {
            log::debug!("DecoderPusher: parse_and_send_packets failed, dropping frame");
            frame_data.report_codec_error();
        }

        Some(frame_data)
    }
}

fn parse_and_send_packets(
    decode_context: &mut AVCodecContext,
    parser_context: &mut AVCodecParserContext,
    input_buffer: &[u8],
    frame_id: i64,
) -> Result<(), ()> {
    let mut packet = rsmpeg::avcodec::AVPacket::new();
    let mut parsed_offset = 0;

    packet.set_pts(frame_id);

    while parsed_offset < input_buffer.len() {
        let (packet_ready, consumed) = parser_context
            .parse_packet(decode_context, &mut packet, &input_buffer[parsed_offset..])
            .map_err(|e| {
                log::debug!("Parser error: {:?}", e);
            })?;

        parsed_offset += consumed;

        if packet_ready {
            let mut sent = false;
            while !sent {
                match decode_context.send_packet(Some(&packet)) {
                    Ok(()) => {
                        sent = true;
                    }
                    Err(RsmpegError::DecoderFullError) => {
                        log::trace!("DecoderPusher: decoder full, returning for puller to drain");
                        sent = true;
                    }
                    Err(RsmpegError::DecoderFlushedError) => {
                        log::debug!("Decoder already flushed, skipping packet");
                        sent = true;
                    }
                    Err(e) => {
                        log::debug!("DecoderPusher: send_packet error: {:?}", e);
                        return Err(());
                    }
                }
            }

            packet = rsmpeg::avcodec::AVPacket::new();
            packet.set_pts(frame_id);
        }
    }

    Ok(())
}
