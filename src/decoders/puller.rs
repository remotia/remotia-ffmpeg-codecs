use std::sync::Arc;

use bytes::BufMut;
use log::debug;
use rsmpeg::{
    avcodec::{AVCodecContext},
    avutil::{AVFrame},
    error::RsmpegError,
    swscale::SwsContext,
};

use remotia::{
    buffers::BufferMut,
    traits::{BorrowMutFrameProperties, FrameError, FrameProcessor},
};

use async_trait::async_trait;
use tokio::sync::Mutex;

pub struct DecoderPuller<K, E> {
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaling_context: SwsContext,
    pub(super) decoded_buffer_key: K,
    pub(super) drain_error: E,

    pub(super) output_avframe: AVFrame,
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
                    let output_avframe = &mut self.output_avframe;
                    output_avframe.set_pts(codec_avframe.pts);

                    self.scaling_context
                        .scale_frame(&codec_avframe, 0, codec_avframe.height, output_avframe)
                        .unwrap();

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