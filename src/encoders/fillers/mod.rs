//! [`AVFrameFiller`] trait and built-in fillers for common pixel formats.
//!
//! Fillers are responsible for copying pixel data from the pipeline's frame data type
//! into the FFmpeg [`AVFrame`] that the encoder will consume. Each filler is generic
//! over a buffer key type `K` that identifies which buffer slot in the frame data
//! contains the pixel data.
//!
//! # Available fillers
//!
//! - [`rgba::RGBAFrameFiller`] — for interleaved RGBA pixel data
//! - [`yuv420p::YUV420PFrameFiller`] — for planar YUV420P pixel data

use rsmpeg::avutil::AVFrame;

pub mod rgba;
pub mod yuv420p;

/// Trait for copying pixel data from frame data into an FFmpeg [`AVFrame`].
///
/// Implementations are provided for common pixel formats (see [`rgba::RGBAFrameFiller`]
/// and [`yuv420p::YUV420PFrameFiller`]). Custom fillers can be created for other
/// pixel formats by implementing this trait.
///
/// The `fill` method returns `false` if the source buffer is not available in the
/// frame data, signalling to the [`EncoderPusher`](super::EncoderPusher) that the
/// frame should be dropped.
pub trait AVFrameFiller<F> {
    /// Copies pixel data from `frame_data` into `avframe`.
    ///
    /// Returns `true` if the copy succeeded, or `false` if the source buffer
    /// was not available (e.g. the key doesn't exist in the frame data).
    fn fill(&mut self, frame_data: &F, avframe: &mut AVFrame) -> bool;
}
