//! Pixel format conversion and scaling via FFmpeg's `swscale`.
//!
//! This module provides [`Scaler`] and its [`ScalerBuilder`] for converting between
//! pixel formats (e.g. RGBA → YUV420P) and optionally resizing frames. A scaler is
//! required by both [`EncoderBuilder`](crate::encoders::EncoderBuilder) and
//! [`DecoderBuilder`](crate::decoders::DecoderBuilder) to ensure frames are in the
//! correct pixel format for the codec.
//!
//! # Example
//!
//! ```no_run
//! use remotia_ffmpeg_codecs::scaling::ScalerBuilder;
//! use remotia_ffmpeg_codecs::ffi;
//!
//! // RGBA input → YUV420P output (typical for H.264 encoding)
//! let scaler = ScalerBuilder::new()
//!     .input_width(1920)
//!     .input_height(1080)
//!     .input_pixel_format(ffi::AV_PIX_FMT_RGBA)
//!     .output_pixel_format(ffi::AV_PIX_FMT_YUV420P)
//!     .build();
//!
//! // YUV420P input → RGBA output (typical for H.264 decoding)
//! let scaler = ScalerBuilder::new()
//!     .input_width(1920)
//!     .input_height(1080)
//!     .input_pixel_format(ffi::AV_PIX_FMT_YUV420P)
//!     .output_pixel_format(ffi::AV_PIX_FMT_RGBA)
//!     .build();
//! ```

use crate::{builder::unwrap_mandatory, ffi};
use rsmpeg::{avutil::AVFrame, swscale::SwsContext};

/// Builder for constructing a [`Scaler`].
///
/// # Mandatory fields
///
/// - [`input_width`](ScalerBuilder::input_width)
/// - [`input_height`](ScalerBuilder::input_height)
/// - [`input_pixel_format`](ScalerBuilder::input_pixel_format)
/// - [`output_pixel_format`](ScalerBuilder::output_pixel_format)
///
/// # Optional fields (with defaults)
///
/// - [`output_width`](ScalerBuilder::output_width) — defaults to `input_width`
/// - [`output_height`](ScalerBuilder::output_height) — defaults to `input_height`
/// - [`scaling_flags`](ScalerBuilder::scaling_flags) — defaults to `SWS_BILINEAR`
pub struct ScalerBuilder {
    input_width: Option<i32>,
    input_height: Option<i32>,
    input_pixel_format: Option<ffi::AVPixelFormat>,
    output_width: Option<i32>,
    output_height: Option<i32>,
    output_pixel_format: Option<ffi::AVPixelFormat>,
    scaling_flags: Option<u32>,
}

impl Default for ScalerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScalerBuilder {
    /// Creates a new scaler builder with all fields unset.
    pub fn new() -> Self {
        Self {
            input_width: None,
            input_height: None,
            input_pixel_format: None,
            output_width: None,
            output_height: None,
            output_pixel_format: None,
            scaling_flags: None,
        }
    }

    builder_set!(input_width, i32);
    builder_set!(input_height, i32);
    builder_set!(output_width, i32);
    builder_set!(output_height, i32);
    builder_set!(input_pixel_format, ffi::AVPixelFormat);
    builder_set!(output_pixel_format, ffi::AVPixelFormat);
    builder_set!(scaling_flags, u32);

    /// Builds the [`Scaler`], panicking if any mandatory field is missing.
    ///
    /// Allocates the FFmpeg `SwsContext` and pre-allocates input/output [`AVFrame`]s
    /// with the specified dimensions and pixel formats.
    pub fn build(self) -> Scaler {
        let input_width = unwrap_mandatory(self.input_width);
        let input_height = unwrap_mandatory(self.input_height);
        let input_pixel_format = unwrap_mandatory(self.input_pixel_format);

        let output_width = self.output_width.unwrap_or(input_width);
        let output_height = self.output_height.unwrap_or(input_height);
        let output_pixel_format = unwrap_mandatory(self.output_pixel_format);

        let scaling_flags = self.scaling_flags.unwrap_or(ffi::SWS_BILINEAR);

        let sws_context = {
            SwsContext::get_context(
                input_width,
                input_height,
                input_pixel_format,
                output_width,
                output_height,
                output_pixel_format,
                scaling_flags,
                None,
                None,
                None,
            )
            .unwrap()
        };

        let input_avframe = {
            let mut avframe = AVFrame::new();
            avframe.set_format(input_pixel_format);
            avframe.set_width(input_width);
            avframe.set_height(input_height);
            avframe.alloc_buffer().unwrap();
            avframe
        };

        let output_avframe = {
            let mut avframe = AVFrame::new();
            avframe.set_format(output_pixel_format);
            avframe.set_width(output_width);
            avframe.set_height(output_height);
            avframe.alloc_buffer().unwrap();
            avframe
        };

        Scaler {
            input_frame: input_avframe,
            scaled_frame: output_avframe,
            sws_context,
        }
    }
}

/// FFmpeg scaler that converts pixel formats and optionally resizes frames.
///
/// A `Scaler` owns an input [`AVFrame`] and an output (scaled) [`AVFrame`]. Frame data
/// is copied into the input frame, then [`scale`](Scaler::scale) performs the conversion
/// into the output frame. Alternatively, [`scale_input`](Scaler::scale_input) accepts an
/// external [`AVFrame`] directly (used by the decoder).
///
/// Typically created via [`ScalerBuilder`] and passed into
/// [`EncoderBuilder::scaler`](crate::encoders::EncoderBuilder::scaler) or
/// [`DecoderBuilder::scaler`](crate::decoders::DecoderBuilder::scaler).
pub struct Scaler {
    sws_context: SwsContext,
    input_frame: AVFrame,
    scaled_frame: AVFrame,
}
impl Scaler {
    /// Scales the internal input frame into the internal output frame.
    ///
    /// Call this after writing pixel data into [`input_frame_mut`](Scaler::input_frame_mut).
    /// The result is available via [`scaled_frame`](Scaler::scaled_frame) or
    /// [`scaled_frame_mut`](Scaler::scaled_frame_mut).
    pub fn scale(&mut self) {
        let input_frame = &self.input_frame;
        let scaled_frame = &mut self.scaled_frame;

        self.sws_context
            .scale_frame(input_frame, 0, input_frame.height, scaled_frame)
            .unwrap();
    }

    /// Scales an external [`AVFrame`] into the internal output frame.
    ///
    /// This is used by [`DecoderPusher`](crate::decoders::DecoderPusher) which receives
    /// decoded frames directly from the FFmpeg codec context rather than filling an
    /// internal buffer.
    pub fn scale_input(&mut self, input_frame: &AVFrame) {
        let scaled_frame = &mut self.scaled_frame;

        self.sws_context
            .scale_frame(input_frame, 0, input_frame.height, scaled_frame)
            .unwrap();
    }

    /// Returns a shared reference to the internal input frame.
    pub fn input_frame(&self) -> &AVFrame {
        &self.input_frame
    }

    /// Returns a shared reference to the scaled (output) frame.
    pub fn scaled_frame(&self) -> &AVFrame {
        &self.scaled_frame
    }

    /// Returns a mutable reference to the internal input frame.
    ///
    /// Write raw pixel data here before calling [`scale`](Scaler::scale).
    pub fn input_frame_mut(&mut self) -> &mut AVFrame {
        &mut self.input_frame
    }

    /// Returns a mutable reference to the scaled (output) frame.
    pub fn scaled_frame_mut(&mut self) -> &mut AVFrame {
        &mut self.scaled_frame
    }
}
