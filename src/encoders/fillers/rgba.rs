use remotia::{traits::BorrowFrameProperties, buffers::BytesMut};
use rsmpeg::avutil::AVFrame;

use super::AVFrameFiller;

pub struct RGBAFrameFiller<K> {
    pub(super) rgba_buffer_key: K,
}

impl<K> RGBAFrameFiller<K> {
    pub fn new(rgba_buffer_key: K) -> Self {
        Self { rgba_buffer_key }
    }
}

impl<F, K> AVFrameFiller<F> for RGBAFrameFiller<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    fn fill(&mut self, frame_data:&F, avframe: &mut AVFrame) {
        let source_buffer = frame_data.get_ref(&self.rgba_buffer_key).unwrap();

        let linesize = avframe.linesize;
        let height = avframe.height as usize;

        let linesize = linesize[0] as usize;
        let data = unsafe { std::slice::from_raw_parts_mut(avframe.data[0], height * linesize) };

        data.copy_from_slice(source_buffer);
    }
}