use std::sync::Arc;

use remotia::traits::FrameProcessor;
use rsmpeg::avcodec::AVCodecContext;

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::scaling::Scaler;

use super::{fillers::AVFrameFiller, FFMpegEncode};

pub struct EncoderPusher<T> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) filler: T,
}

#[async_trait]
impl<F, T> FrameProcessor<F> for EncoderPusher<T>
where
    T: AVFrameFiller<F> + Send,
    F: FFMpegEncode + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let mut encode_context = self.encode_context.lock().await;

        let input_avframe = self.scaler.input_frame_mut();
        self.filler.fill(&frame_data, input_avframe);

        self.scaler.scale();
        self.scaler.scaled_frame_mut().set_pts(frame_data.get_frame_id() as i64);

        encode_context
            .send_frame(Some(&self.scaler.scaled_frame()))
            .unwrap();

        Some(frame_data)
    }
}
