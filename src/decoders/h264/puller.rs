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

pub struct H264DecoderPuller<K, E> {
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaling_context: SwsContext,
    pub(super) decoded_buffer_key: K,
    pub(super) drain_error: E,
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for H264DecoderPuller<K, E>
where
    K: Send + Copy,
    E: Send + Copy,
    F: BorrowMutFrameProperties<K, BufferMut> + FrameError<E> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        loop {
            let mut decode_context = self.decode_context.lock().await;
            match decode_context.receive_frame() {
                Ok(yuv_frame) => {
                    let mut rgba_frame = AVFrame::new();
                    rgba_frame.set_format(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGBA);
                    rgba_frame.set_width(yuv_frame.width);
                    rgba_frame.set_height(yuv_frame.height);
                    rgba_frame.set_pts(yuv_frame.pts);
                    rgba_frame.alloc_buffer().unwrap();

                    self.scaling_context
                        .scale_frame(&yuv_frame, 0, yuv_frame.height, &mut rgba_frame)
                        .unwrap();

                    let linesize = rgba_frame.linesize;
                    let height = rgba_frame.height as usize;

                    let linesize = linesize[0] as usize;
                    let data = unsafe { std::slice::from_raw_parts(rgba_frame.data[0], height * linesize) };

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