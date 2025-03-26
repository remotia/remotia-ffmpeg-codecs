use std::sync::Arc;

use rsmpeg::avcodec::AVCodecContext;

use remotia::traits::{FrameError, FrameProcessor};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{scaling::Scaler, FFMpegCodec};

pub struct DecoderPuller {
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
}

impl DecoderPuller {
    pub fn flusher_on<E>(&self, flush_error: E) -> DecoderFlusher<E> {
        DecoderFlusher {
            decode_context: self.decode_context.clone(),
            flush_error,
            used: false,
        }
    }
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
                log::trace!("Received AVFrame: {:#?}", codec_avframe);
                frame_data.set_frame_id(codec_avframe.pts);

                self.scaler.scale_input(&codec_avframe);

                let output_avframe = &mut self.scaler.scaled_frame_mut();

                let linesize = output_avframe.linesize;
                let height = output_avframe.height as usize;

                let linesize = linesize[0] as usize;
                let data = unsafe { std::slice::from_raw_parts(output_avframe.data[0], height * linesize) };

                frame_data.write_decoded_buffer(data);
            }
            Err(error) => {
                log::debug!("Decoding context returned error '{error:?}'");
                frame_data.report_codec_error(error);
            }
        }

        Some(frame_data)
    }
}

pub struct DecoderFlusher<E> {
    pub(crate) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(crate) flush_error: E,
    used: bool,
}

#[async_trait]
impl<F, E> FrameProcessor<F> for DecoderFlusher<E>
where
    E: Send + Copy + std::cmp::PartialEq + std::fmt::Debug,
    F: FrameError<E> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        if self.used {
            log::warn!("Attempt to double-flush the decoder");
            return Some(frame_data);
        }

        if let Some(error) = frame_data.get_error() {
            if error == self.flush_error {
                log::debug!("Received flush error {error:?}, flushing decode context...");
                self.decode_context.lock().await.send_packet(None).unwrap();
                self.used = true;
            }
        }

        Some(frame_data)
    }
}
