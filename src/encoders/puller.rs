use std::sync::Arc;

use remotia::traits::{FrameError, FrameProcessor};
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use async_trait::async_trait;

use tokio::sync::Mutex;

use super::FFMpegEncode;

pub struct EncoderPuller {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
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
impl<'a, F> FrameProcessor<F> for EncoderPuller
where
    F: FFMpegEncode + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        loop {
            let mut encode_context = self.encode_context.lock().await;

            let packet = match encode_context.receive_packet() {
                Ok(packet) => {
                    // debug!("Received packet of size {}", packet.size);
                    packet
                }
                Err(RsmpegError::EncoderDrainError) => {
                    log::debug!("Drain error, breaking the loop");
                    break;
                }
                Err(RsmpegError::EncoderFlushedError) => {
                    log::debug!("Flushed error, breaking the loop");
                    frame_data.report_flush_error();
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };

            let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };

            frame_data.set_frame_id(packet.pts);
            frame_data.write_packet_data(data);
        }
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
                log::debug!("Received flush error, flushing encode context...");
                self.encode_context.lock().await.send_frame(None).unwrap();
            }
        }

        Some(frame_data)
    }
}
