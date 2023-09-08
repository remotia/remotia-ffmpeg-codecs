use std::sync::Arc;

use remotia::traits::FrameProcessor;
use rsmpeg::avcodec::AVCodecContext;

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::scaling::Scaler;

use super::fillers::AVFrameFiller;

pub struct EncoderPusher<T> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) filler: T,
}

#[async_trait]
impl<F, T> FrameProcessor<F> for EncoderPusher<T>
where
    T: AVFrameFiller<F> + Send,
    F: Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let pts = 0 as i64; // TODO: Implement timestamp

        let mut encode_context = self.encode_context.lock().await;

        let input_avframe = self.scaler.input_frame_mut();
        input_avframe.set_pts(pts);

        self.filler.fill(&frame_data, input_avframe);

        self.scaler.scale();

        encode_context
            .send_frame(Some(&self.scaler.scaled_frame()))
            .unwrap();

        Some(frame_data)
    }
}
