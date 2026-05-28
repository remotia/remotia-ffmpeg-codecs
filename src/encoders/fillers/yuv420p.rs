//! [`YUV420PFrameFiller`] — filler for planar YUV420P pixel data.

use remotia::{buffers::BytesMut, traits::BorrowFrameProperties};
use rsmpeg::avutil::AVFrame;

use super::AVFrameFiller;

/// [`AVFrameFiller`] that copies planar YUV420P pixel data into an FFmpeg [`AVFrame`].
///
/// The filler reads a [`BytesMut`] buffer identified by `yuv420p_buffer_key` from the
/// frame data and splits it into the Y, U, and V planes of the AVFrame. The source
/// buffer is expected to contain the planes concatenated in Y-U-V order with the
/// following sizes:
///
/// - Y plane: `height × linesize[0]`
/// - U plane: `(height/2) × linesize[1]`
/// - V plane: `(height/2) × linesize[2]`
///
/// This is typically used when the pipeline's captured frames are already in YUV420P
/// format (e.g. from a Y4M source) and can be fed directly to the encoder without
/// pixel format conversion.
///
/// # Example
///
/// ```no_run
/// use remotia_ffmpeg_codecs::encoders::fillers::yuv420p::YUV420PFrameFiller;
///
/// let filler = YUV420PFrameFiller::new(MyBufferKey::YuvFrame);
/// ```
pub struct YUV420PFrameFiller<K> {
    pub(super) yuv420p_buffer_key: K,
}

impl<K> YUV420PFrameFiller<K> {
    /// Creates a new YUV420P filler that reads pixel data from the buffer identified
    /// by `yuv420p_buffer_key`.
    pub fn new(yuv420p_buffer_key: K) -> Self {
        Self { yuv420p_buffer_key }
    }
}

impl<F, K> AVFrameFiller<F> for YUV420PFrameFiller<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    fn fill(&mut self, frame_data: &F, avframe: &mut AVFrame) -> bool {
        let Some(source_buffer) = frame_data.get_ref(&self.yuv420p_buffer_key) else {
            return false;
        };

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
        true
    }
}
