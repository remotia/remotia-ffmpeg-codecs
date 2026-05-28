//! [`DecoderPuller`] — the draining side of the decoder.

use std::sync::Arc;

use rsmpeg::avcodec::AVCodecContext;

use remotia::pipeline::PipelineHandle;
use remotia::traits::FrameProcessor;
use tokio::sync::mpsc::UnboundedReceiver;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::FFMpegCodec;

/// The puller half of the FFmpeg decoder, responsible for receiving decoded frames.
///
/// `DecoderPuller` implements [`FrameProcessor`] and works in concert with
/// [`DecoderPusher`](super::DecoderPusher). It receives scaled pixel data from the
/// pusher through an internal unbounded channel and writes it into the frame data via
/// [`FFMpegCodec::write_decoded_buffer`].
///
/// If no decoded data is available yet, the frame is passed through unchanged. When
/// the channel is closed (meaning the pusher has finished flushing and dropped its
/// sender), the puller optionally requests a pipeline shutdown via the
/// [`PipelineHandle`] and returns `None` to stop processing.
pub struct DecoderPuller {
    pub(super) _decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) frame_rx: UnboundedReceiver<Vec<u8>>,
    pub(super) pipeline_handle: Option<PipelineHandle>,
}

#[async_trait]
impl<F> FrameProcessor<F> for DecoderPuller
where
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        match self.frame_rx.try_recv() {
            Ok(decoded_data) => {
                frame_data.write_decoded_buffer(&decoded_data);
                Some(frame_data)
            }
            Err(_) => {
                if self.frame_rx.is_closed() {
                    log::debug!("DecoderPuller: decoded frame channel closed, decoder is flushed");
                    if let Some(handle) = &self.pipeline_handle {
                        handle.request_shutdown();
                    }
                    None
                } else {
                    Some(frame_data)
                }
            }
        }
    }
}
