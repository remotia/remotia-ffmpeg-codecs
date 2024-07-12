use std::sync::Arc;

use log::debug;
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use remotia::{
    buffers::{BufMut, BytesMut},
    traits::{BorrowMutFrameProperties, FrameError, FrameProcessor, FrameProperties},
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::scaling::Scaler;

pub struct DecoderPuller<K, E, P> {
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) decoded_buffer_key: K,
    pub(super) drain_error: E,
    pub(super) frame_id_prop: P,
}

#[async_trait]
impl<F, K, E, P> FrameProcessor<F> for DecoderPuller<K, E, P>
where
    K: Send + Copy,
    E: Send + Copy,
    P: Send + Copy,
    F: FrameProperties<P, u128> + BorrowMutFrameProperties<K, BytesMut> + FrameError<E> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        loop {
            let mut decode_context = self.decode_context.lock().await;
            match decode_context.receive_frame() {
                Ok(codec_avframe) => {
                    log::trace!("Received AVFrame: {:#?}", codec_avframe);
                    let frame_id = codec_avframe.pts as u128;

                    unsafe {
                        let raw = codec_avframe.as_ptr();
                        // let raw = decode_context.as_ptr();
                        // let received_frame_id = (*raw).pts;
                        log::trace!("Received raw frame: {:#?}", *raw);
                        // log::debug!("Received frame id: {}", received_frame_id);
                    }

                    self.scaler.scale_input(&codec_avframe);

                    let output_avframe = &mut self.scaler.scaled_frame_mut();

                    let linesize = output_avframe.linesize;
                    let height = output_avframe.height as usize;

                    let linesize = linesize[0] as usize;
                    let data = unsafe { std::slice::from_raw_parts(output_avframe.data[0], height * linesize) };

                    let decoded_buffer = frame_data.get_mut_ref(&self.decoded_buffer_key).unwrap();
                    decoded_buffer.put(data);

                    frame_data.set(self.frame_id_prop, frame_id);

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
