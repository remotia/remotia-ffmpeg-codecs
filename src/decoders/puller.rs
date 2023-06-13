use std::sync::Arc;

use bytes::BufMut;
use log::debug;
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use remotia::{
    buffers::BufferMut,
    traits::{BorrowMutFrameProperties, FrameError, FrameProcessor},
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::scaling::Scaler;

pub struct DecoderPuller<K, E> {
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) decoded_buffer_key: K,
    pub(super) drain_error: E,
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for DecoderPuller<K, E>
where
    K: Send + Copy,
    E: Send + Copy,
    F: BorrowMutFrameProperties<K, BufferMut> + FrameError<E> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        loop {
            let mut decode_context = self.decode_context.lock().await;
            match decode_context.receive_frame() {
                Ok(codec_avframe) => {
                    self.scaler.scale_input(&codec_avframe);

                    let output_avframe = &mut self.scaler.scaled_frame_mut();
                    output_avframe.set_pts(codec_avframe.pts);

                    let linesize = output_avframe.linesize;
                    let height = output_avframe.height as usize;

                    let linesize = linesize[0] as usize;
                    let data = unsafe { std::slice::from_raw_parts(output_avframe.data[0], height * linesize) };

                    let decoded_buffer = frame_data.get_mut_ref(&self.decoded_buffer_key).unwrap();
                    decoded_buffer.put(data);

                    break;
                }
                Err(RsmpegError::DecoderDrainError) => {
                    debug!("No frames to be pulled");
                    frame_data.report_error(self.drain_error);
                    break;
                }
                Err(RsmpegError::DecoderFlushedError) => {
                    panic!("Decoder has been flushed unexpectedly");
                }
                Err(e) => panic!("{:?}", e),
            }
        }

        Some(frame_data)
    }
}
