//! FFmpeg encoder integration for remotia pipelines.
//!
//! This module provides [`EncoderBuilder`] which creates an
//! ([`EncoderPusher`], [`EncoderPuller`]) pair. The pusher feeds raw frames into the
//! FFmpeg encoder, and the puller drains encoded packets from it. They must be placed in
//! **separate pipeline components** linked together so that the puller can receive packets
//! while the pusher continues encoding.
//!
//! # Architecture
//!
//! ```text
//! [EncoderPusher]  ──shared AVCodecContext──►  [EncoderPuller]
//!   (component 1)                              (component 2)
//! ```
//!
//! The pusher and puller share an `Arc<Mutex<AVCodecContext>>` and communicate
//! back-pressure via a [`Notify`] channel: when the encoder's internal buffer is full,
//! the pusher waits for the puller to drain a packet before retrying.
//!
//! # Example
//!
//! ```no_run
//! use remotia_ffmpeg_codecs::encoders::EncoderBuilder;
//! use remotia_ffmpeg_codecs::encoders::fillers::rgba::RGBAFrameFiller;
//! use remotia_ffmpeg_codecs::scaling::ScalerBuilder;
//! use remotia_ffmpeg_codecs::options::Options;
//! use remotia_ffmpeg_codecs::ffi;
//!
//! let scaler = ScalerBuilder::new()
//!     .input_width(1920)
//!     .input_height(1080)
//!     .input_pixel_format(ffi::AV_PIX_FMT_RGBA)
//!     .output_pixel_format(ffi::AV_PIX_FMT_YUV420P)
//!     .build();
//!
//! let (encoder_pusher, encoder_puller) = EncoderBuilder::new()
//!     .codec_id("libx264")
//!     .filler(RGBAFrameFiller::new(MyBufferKey::RgbaFrame))
//!     .scaler(scaler)
//!     .options(Options::new().set("crf", "26").set("tune", "zerolatency"))
//!     .build();
//! ```
//!
//! [`Notify`]: tokio::sync::Notify

use std::{ffi::CString, ptr::NonNull, sync::Arc};

use rsmpeg::avcodec::{AVCodec, AVCodecContext};

use tokio::sync::{Mutex, Notify};

use crate::{builder::unwrap_mandatory, ffi, scaling::Scaler};

use super::options::Options;

pub mod fillers;
mod puller;
mod pusher;

pub use puller::*;
pub use pusher::*;

/// Builder for constructing an ([`EncoderPusher`], [`EncoderPuller`]) pair.
///
/// # Mandatory fields
///
/// - [`codec_id`](EncoderBuilder::codec_id) — FFmpeg encoder name (e.g. `"libx264"`, `"libvpx-vp9"`)
/// - [`filler`](EncoderBuilder::filler) — an [`AVFrameFiller`] that copies frame data into the encoder's input AVFrame
/// - [`scaler`](EncoderBuilder::scaler) — a [`Scaler`] for pixel format conversion
///
/// # Optional fields
///
/// - [`options`](EncoderBuilder::options) — codec-specific options (defaults to empty)
///
/// # Type parameter
///
/// The generic parameter `T` is the filler type, which implements [`AVFrameFiller`]
/// for the specific frame data type used in the pipeline.
///
/// [`AVFrameFiller`]: fillers::AVFrameFiller
pub struct EncoderBuilder<T> {
    codec_id: Option<String>,
    filler: Option<T>,
    options: Option<Options>,
    scaler: Option<Scaler>,
}

impl<T> Default for EncoderBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> EncoderBuilder<T> {
    /// Creates a new encoder builder with all fields unset.
    pub fn new() -> Self {
        Self {
            codec_id: None,
            filler: None,
            options: None,
            scaler: None,
        }
    }

    builder_set!(filler, T);
    builder_set!(options, Options);
    builder_set!(scaler, Scaler);

    /// Sets the FFmpeg encoder name (e.g. `"libx264"`, `"libvpx-vp9"`).
    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }

    /// Builds the encoder, returning an ([`EncoderPusher`], [`EncoderPuller`]) pair.
    ///
    /// The encoder context is configured with:
    /// - Width, height, and pixel format from the [`Scaler`]'s output frame
    /// - Time base of `1/60000` and framerate of `60/1`
    /// - The provided [`Options`] converted to an `AVDictionary`
    ///
    /// # Panics
    ///
    /// Panics if any mandatory field is missing, if the codec is not found, or if the
    /// codec context cannot be opened.
    pub fn build(self) -> (EncoderPusher<T>, EncoderPuller) {
        let codec_id = unwrap_mandatory(self.codec_id);
        let options = self.options.unwrap_or_default();

        let scaler = unwrap_mandatory(self.scaler);

        let encode_context = {
            let codec_id_string = CString::new(codec_id).unwrap();
            let encoder = AVCodec::find_encoder_by_name(&codec_id_string).unwrap();
            let mut encode_context = AVCodecContext::new(&encoder);
            encode_context.set_width(scaler.scaled_frame().width);
            encode_context.set_height(scaler.scaled_frame().height);
            encode_context.set_pix_fmt(scaler.scaled_frame().format);
            encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 * 1000 });
            encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
            let mut encode_context = unsafe {
                let raw_encode_context = encode_context.into_raw().as_ptr();
                AVCodecContext::from_raw(NonNull::new(raw_encode_context).unwrap())
            };

            let options_dict = options.to_av_dict();

            encode_context.open(Some(options_dict)).unwrap();

            Arc::new(Mutex::new(encode_context))
        };

        let filler = unwrap_mandatory(self.filler);
        let packet_drained = Arc::new(Notify::new());

        (
            EncoderPusher {
                encode_context: encode_context.clone(),
                scaler,
                filler,
                eof_processed: false,
                packet_drained: packet_drained.clone(),
            },
            EncoderPuller {
                encode_context: encode_context.clone(),
                flushed: false,
                packet_drained,
            },
        )
    }
}
