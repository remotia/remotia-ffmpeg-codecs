use std::sync::Arc;

use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use remotia::traits::FrameProcessor;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{scaling::Scaler, FFMpegCodec};

pub struct DecoderPuller {
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
}

#[async_trait]
impl<F> FrameProcessor<F> for DecoderPuller
where
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut decode_context = self.decode_context.lock().await;
        match decode_context.receive_frame() {
            Ok(codec_avframe) => {
                frame_data.set_frame_id(codec_avframe.pts);

                self.scaler.scale_input(&codec_avframe);

                let output_avframe = &mut self.scaler.scaled_frame_mut();

                let linesize = output_avframe.linesize;
                let height = output_avframe.height as usize;

                let linesize = linesize[0] as usize;
                let data =
                    unsafe { std::slice::from_raw_parts(output_avframe.data[0], height * linesize) };

                frame_data.write_decoded_buffer(data);
            }
            Err(RsmpegError::DecoderDrainError) => {
                log::trace!("No frames to be pulled");
                frame_data.report_decoder_drain_error();
            }
            Err(RsmpegError::DecoderFlushedError) => {
                log::debug!("Decoder has been flushed");
                frame_data.report_decoder_drain_error();
            }
            Err(e) => {
                log::warn!("DecoderPuller: receive_frame error: {:?}", e);
                frame_data.report_codec_error();
            }
        }

        Some(frame_data)
    }
}
