//! [`EncoderPusher`] — the feeding side of the encoder pusher/puller pair.

use std::sync::Arc;

use remotia::traits::FrameProcessor;
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use async_trait::async_trait;

use tokio::sync::{Mutex, Notify};

use crate::{scaling::Scaler, FFMpegCodec};

use super::fillers::AVFrameFiller;

/// The pusher half of the FFmpeg encoder, responsible for feeding raw frames into the
/// encoding context.
///
/// `EncoderPusher` implements [`FrameProcessor`] and works in concert with
/// [`EncoderPuller`](super::EncoderPuller). On each call to [`process`](FrameProcessor::process):
///
/// 1. The [`AVFrameFiller`] copies pixel data from the incoming frame into the
///    [`Scaler`]'s input frame.
/// 2. The scaler converts the pixel format (and optionally resizes).
/// 3. The scaled frame is sent to the FFmpeg encoder via `send_frame`.
/// 4. If the encoder's internal buffer is full (`SendFrameAgainError`), the pusher
///    waits for the puller to drain a packet via the shared [`Notify`] channel.
///
/// When an EOF frame is received ([`FFMpegCodec::is_eof`] returns `true`), the pusher
/// sends a `None` frame to flush the encoder and then stops processing further frames.
pub struct EncoderPusher<T> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) filler: T,
    pub(super) eof_processed: bool,
    pub(super) packet_drained: Arc<Notify>,
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

        loop {
            match encode_context.send_frame(Some(self.scaler.scaled_frame())) {
                Ok(()) => {
                    log::debug!("EncoderPusher: send_frame accepted frame_id={}", frame_id);
                    break;
                }
                Err(RsmpegError::SendFrameAgainError) => {
                    log::debug!("EncoderPusher: send_frame AGAIN for frame_id={}, waiting for puller to drain", frame_id);
                    drop(encode_context);
                    self.packet_drained.notified().await;
                    encode_context = self.encode_context.lock().await;
                }
                Err(e) => {
                    log::warn!("EncoderPusher: send_frame error for frame_id={}: {:?}", frame_id, e);
                    break;
                }
            }
        }

        Some(frame_data)
    }
}
