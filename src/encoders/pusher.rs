use std::sync::Arc;

use remotia::traits::FrameProcessor;
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::{scaling::Scaler, FFMpegCodec};

use super::fillers::AVFrameFiller;

pub struct EncoderPusher<T> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) filler: T,
    pub(super) eof_processed: bool,
}

#[async_trait]
impl<F, T> FrameProcessor<F> for EncoderPusher<T>
where
    T: AVFrameFiller<F> + Send,
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        if self.eof_processed {
            return None;
        }

        let frame_id = frame_data.get_frame_id();

        let mut encode_context = self.encode_context.lock().await;

        if frame_data.is_eof() {
            log::debug!("EncoderPusher: EOF signal received, flushing encoder");
            encode_context.send_frame(None).ok();
            self.eof_processed = true;
            return Some(frame_data);
        }

        let input_avframe = self.scaler.input_frame_mut();
        if !self.filler.fill(&frame_data, input_avframe) {
            log::warn!("EncoderPusher: Filler rejected frame_id={}", frame_id);
            return None;
        }

        self.scaler.scale();
        self.scaler
            .scaled_frame_mut()
            .set_pts(frame_data.get_frame_id());

        log::debug!("EncoderPusher: send_frame frame_id={}", frame_id);

        match encode_context.send_frame(Some(self.scaler.scaled_frame())) {
            Ok(()) => {
                log::debug!("EncoderPusher: send_frame accepted frame_id={}", frame_id);
            }
            Err(RsmpegError::SendFrameAgainError) => {
                log::warn!("EncoderPusher: send_frame AGAIN for frame_id={}, DROPPING", frame_id);
            }
            Err(e) => {
                log::warn!("EncoderPusher: send_frame error for frame_id={}: {:?}", frame_id, e);
            }
        }

        Some(frame_data)
    }
}
