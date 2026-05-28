//! [`RGBAFrameFiller`] — filler for interleaved RGBA pixel data.

use remotia::{traits::BorrowFrameProperties, buffers::BytesMut};
use rsmpeg::avutil::AVFrame;

use super::AVFrameFiller;

/// [`AVFrameFiller`] that copies interleaved RGBA pixel data into an FFmpeg [`AVFrame`].
///
/// The filler reads a [`BytesMut`] buffer identified by `rgba_buffer_key` from the
/// frame data and copies it into the AVFrame's first data plane. This is typically
/// used when the pipeline's captured frames are in RGBA format and need to be
/// converted to YUV420P by the [`Scaler`](crate::scaling::Scaler) before encoding.
///
/// # Example
///
/// ```no_run
/// use remotia_ffmpeg_codecs::encoders::fillers::rgba::RGBAFrameFiller;
///
/// let filler = RGBAFrameFiller::new(MyBufferKey::RgbaFrame);
/// ```
pub struct RGBAFrameFiller<K> {
    pub(super) rgba_buffer_key: K,
}

impl<K> RGBAFrameFiller<K> {
    /// Creates a new RGBA filler that reads pixel data from the buffer identified
    /// by `rgba_buffer_key`.
    pub fn new(rgba_buffer_key: K) -> Self {
        Self { rgba_buffer_key }
    }
}

impl<F, K> AVFrameFiller<F> for RGBAFrameFiller<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    fn fill(&mut self, frame_data: &F, avframe: &mut AVFrame) -> bool {
        let Some(source_buffer) = frame_data.get_ref(&self.rgba_buffer_key) else {
            return false;
        };

        let linesize = avframe.linesize;
        let height = avframe.height as usize;

        let linesize = linesize[0] as usize;
        let data = unsafe { std::slice::from_raw_parts_mut(avframe.data[0], height * linesize) };

        data.copy_from_slice(source_buffer);
        true
    }
}
