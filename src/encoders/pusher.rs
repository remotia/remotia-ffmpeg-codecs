use std::sync::Arc;

use bytes::BytesMut;
use remotia::traits::{BorrowFrameProperties, FrameProcessor};
use rsmpeg::{avcodec::AVCodecContext, avutil::AVFrame, swscale::SwsContext};

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::ffi;

pub struct EncoderPusher<K> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaling_context: SwsContext,
    pub(super) rgba_buffer_key: K,

    pub(super) input_avframe: AVFrame,
}

#[async_trait]
impl<F, K> FrameProcessor<F> for EncoderPusher<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let pts = 0 as i64; // TODO: Implement timestamp

        let mut encode_context = self.encode_context.lock().await;

        let input_avframe = &mut self.input_avframe;
        input_avframe.set_pts(pts);

        let linesize = input_avframe.linesize;
        let height = encode_context.height as usize;

        let linesize = linesize[0] as usize;
        let data = unsafe { std::slice::from_raw_parts_mut(input_avframe.data[0], height * linesize) };

        data.copy_from_slice(frame_data.get_ref(&self.rgba_buffer_key).unwrap());

        let mut codec_avframe = AVFrame::new();
        codec_avframe.set_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
        codec_avframe.set_width(encode_context.width);
        codec_avframe.set_height(encode_context.height);
        codec_avframe.set_pts(pts);
        codec_avframe.alloc_buffer().unwrap();

        self.scaling_context
            .scale_frame(&input_avframe, 0, input_avframe.height, &mut codec_avframe)
            .unwrap();

        encode_context.send_frame(Some(&codec_avframe)).unwrap();

        Some(frame_data)
    }
}
