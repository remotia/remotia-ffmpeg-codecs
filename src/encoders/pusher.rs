use std::sync::Arc;

use remotia::{
    buffers::BytesMut,
    traits::{BorrowFrameProperties, FrameProcessor},
};
use rsmpeg::avcodec::AVCodecContext;

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::scaling::Scaler;

pub struct EncoderPusher<K> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) rgba_buffer_key: K,
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

        let input_avframe = self.scaler.input_frame_mut();
        input_avframe.set_pts(pts);

        let linesize = input_avframe.linesize;
        let height = input_avframe.height as usize;

        let linesize = linesize[0] as usize;
        let data = unsafe { std::slice::from_raw_parts_mut(input_avframe.data[0], height * linesize) };

        data.copy_from_slice(frame_data.get_ref(&self.rgba_buffer_key).unwrap());

        self.scaler.scale();

        encode_context
            .send_frame(Some(&self.scaler.scaled_frame()))
            .unwrap();

        Some(frame_data)
    }
}
