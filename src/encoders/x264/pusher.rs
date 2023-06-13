use std::{sync::Arc};

use bytes::BytesMut;
use remotia::traits::{BorrowFrameProperties, FrameProcessor};
use rsmpeg::{
    avcodec::{AVCodecContext},
    avutil::AVFrame,
    swscale::SwsContext,
};

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::{ffi, encoders::utils::avframe::send_avframe};

pub struct X264EncoderPusher<K> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaling_context: SwsContext,

    pub(super) rgba_buffer_key: K,
}

#[async_trait]
impl<F, K> FrameProcessor<F> for X264EncoderPusher<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let pts = 0 as i64; // TODO: Implement timestamp

        let mut encode_context = self.encode_context.lock().await;

        let mut rgba_frame = AVFrame::new();
        rgba_frame.set_format(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGBA);
        rgba_frame.set_width(encode_context.width);
        rgba_frame.set_height(encode_context.height);
        rgba_frame.set_pts(pts);
        rgba_frame.alloc_buffer().unwrap();
        let linesize = rgba_frame.linesize;
        let height = encode_context.height as usize;

        let linesize = linesize[0] as usize;
        let data = unsafe { std::slice::from_raw_parts_mut(rgba_frame.data[0], height * linesize) };

        data.copy_from_slice(frame_data.get_ref(&self.rgba_buffer_key).unwrap());

        let mut yuv_frame = AVFrame::new();
        yuv_frame.set_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
        yuv_frame.set_width(encode_context.width);
        yuv_frame.set_height(encode_context.height);
        yuv_frame.set_pts(pts);
        yuv_frame.alloc_buffer().unwrap();

        self.scaling_context.scale_frame(&rgba_frame, 0, rgba_frame.height, &mut yuv_frame).unwrap();

        send_avframe(&mut encode_context, yuv_frame);

        Some(frame_data)
    }
}