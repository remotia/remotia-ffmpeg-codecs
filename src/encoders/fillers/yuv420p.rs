use remotia::{buffers::BytesMut, traits::BorrowFrameProperties};
use rsmpeg::avutil::AVFrame;

use super::AVFrameFiller;

pub struct YUV420PFrameFiller<K> {
    pub(super) yuv420p_buffer_key: K,
}

impl<K> YUV420PFrameFiller<K> {
    pub fn new(yuv420p_buffer_key: K) -> Self {
        Self { yuv420p_buffer_key }
    }
}

impl<F, K> AVFrameFiller<F> for YUV420PFrameFiller<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    fn fill(&mut self, frame_data: &F, avframe: &mut AVFrame) {
        let source_buffer = frame_data.get_ref(&self.yuv420p_buffer_key).unwrap();

        let linesize = avframe.linesize;
        let height = avframe.height as usize;

        let y_data = unsafe { std::slice::from_raw_parts_mut(avframe.data[0], height * linesize[0] as usize) };
        let u_data = unsafe { std::slice::from_raw_parts_mut(avframe.data[1], (height / 2) * linesize[1] as usize) };
        let v_data = unsafe { std::slice::from_raw_parts_mut(avframe.data[2], (height / 2) * linesize[2] as usize) };

        let mut written_bytes = 0;
        y_data.copy_from_slice(&source_buffer[..y_data.len()]);
        written_bytes += y_data.len();

        u_data.copy_from_slice(&source_buffer[written_bytes..written_bytes + u_data.len()]);
        written_bytes += u_data.len();

        v_data.copy_from_slice(&source_buffer[written_bytes..written_bytes + v_data.len()]);
    }
}
