use std::sync::Arc;

use remotia::traits::{FrameError, FrameProcessor};
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use async_trait::async_trait;

use tokio::sync::{Mutex, Notify};

use crate::FFMpegCodec;

pub struct EncoderPuller {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) flushed: bool,
    pub(super) packet_drained: Arc<Notify>,
}

impl EncoderPuller {
    pub fn flusher_on<E>(&self, flush_error: E) -> EncoderFlusher<E> {
        EncoderFlusher {
            encode_context: self.encode_context.clone(),
            flush_error,
        }
    }
}

#[async_trait]
impl<F> FrameProcessor<F> for EncoderPuller
where
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        if self.flushed && frame_data.get_packet_data_buffer().is_empty() {
            log::debug!("EncoderPuller: already flushed, no packet data, dropping frame");
            return None;
        }

        let incoming_len = frame_data.get_packet_data_buffer().len();
        let incoming_frame_id = frame_data.get_frame_id();
        log::debug!("EncoderPuller: processing frame_id={}, incoming_packet_data={} bytes", incoming_frame_id, incoming_len);

        let mut packets_received = 0u32;

        loop {
            let mut encode_context = self.encode_context.lock().await;

            let packet = match encode_context.receive_packet() {
                Ok(packet) => packet,
                Err(RsmpegError::EncoderDrainError) => {
                    log::debug!("EncoderPuller: receive_packet => DrainError (no more packets available right now), total_packets={}", packets_received);
                    break;
                }
                Err(RsmpegError::EncoderFlushedError) => {
                    log::debug!("EncoderPuller: receive_packet => FlushedError (encoder fully flushed), total_packets={}", packets_received);
                    frame_data.report_flush_error();
                    self.flushed = true;
                    break;
                }
                Err(e) => {
                    log::warn!("EncoderPuller: receive_packet error: {:?}, total_packets={}", e, packets_received);
                    break;
                }
            };

            let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
            let pts = packet.pts;

            frame_data.set_frame_id(pts);
            frame_data.write_packet_data(data);
            packets_received += 1;
            self.packet_drained.notify_one();

            log::debug!("EncoderPuller: received packet #{}: pts={}, size={} bytes", packets_received, pts, data.len());
        }

        let outgoing_len = frame_data.get_packet_data_buffer().len();
        log::debug!("EncoderPuller: done processing frame_id={}: incoming={} bytes, outgoing={} bytes, packets_received={}", incoming_frame_id, incoming_len, outgoing_len, packets_received);

        Some(frame_data)
    }
}

pub struct EncoderFlusher<E> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(crate) flush_error: E,
}

#[async_trait]
impl<F, E> FrameProcessor<F> for EncoderFlusher<E>
where
    E: Send + Copy + std::cmp::PartialEq,
    F: FrameError<E> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        if let Some(error) = frame_data.get_error() {
            if error == self.flush_error {
                log::debug!("EncoderFlusher: Received flush error, flushing encode context...");
                self.encode_context.lock().await.send_frame(None).ok();
            }
        }

        Some(frame_data)
    }
}
