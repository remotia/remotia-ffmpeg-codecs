use std::sync::Arc;

use remotia::traits::FrameProcessor;
use rsmpeg::avcodec::AVCodecContext;

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::{scaling::Scaler, FFMpegCodec};

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
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut encode_context = self.encode_context.lock().await;

        let input_avframe = self.scaler.input_frame_mut();
        self.filler.fill(&frame_data, input_avframe);

        self.scaler.scale();
        self.scaler
            .scaled_frame_mut()
            .set_pts(frame_data.get_frame_id());

        let send_result = encode_context.send_frame(Some(self.scaler.scaled_frame()));

        if let Err(error) = send_result {
            match error {
                err => {
                    log::warn!("Unhandled codec error during frame send: {}", err);
                    frame_data.report_codec_error();
                },
            }
        }

        Some(frame_data)
    }
}
