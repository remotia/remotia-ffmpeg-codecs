//! FFmpeg decoder integration for remotia pipelines.
//!
//! This module provides [`DecoderBuilder`] which creates a
//! ([`DecoderPusher`], [`DecoderPuller`]) pair. The pusher feeds encoded packets into
//! the FFmpeg decoder, and the puller receives decoded frames from it. They must be
//! placed in **separate pipeline components** linked together.
//!
//! # Architecture
//!
//! Unlike the encoder (which shares an `AVCodecContext` between pusher and puller), the
//! decoder uses an internal **unbounded channel** to pass decoded frame data from the
//! pusher to the puller. This is because FFmpeg's decoder API produces frames on the
//! same thread that sends packets, so the pusher drains all available frames after each
//! `send_packet` call and sends them through the channel.
//!
//! ```text
//! [DecoderPusher]  ──unbounded channel──►  [DecoderPuller]
//!   (component 1)                            (component 2)
//! ```
//!
//! When the input reaches EOF, the pusher flushes the decoder, drops the channel sender,
//! and optionally requests a pipeline shutdown via the [`PipelineHandle`].
//!
//! # Example
//!
//! ```no_run
//! use remotia_ffmpeg_codecs::decoders::DecoderBuilder;
//! use remotia_ffmpeg_codecs::scaling::ScalerBuilder;
//! use remotia_ffmpeg_codecs::ffi;
//!
//! let scaler = ScalerBuilder::new()
//!     .input_width(1920)
//!     .input_height(1080)
//!     .input_pixel_format(ffi::AV_PIX_FMT_YUV420P)
//!     .output_pixel_format(ffi::AV_PIX_FMT_RGBA)
//!     .build();
//!
//! let (decoder_pusher, decoder_puller) = DecoderBuilder::new()
//!     .codec_id("h264")
//!     .scaler(scaler)
//!     .build();
//! ```
//!
//! [`PipelineHandle`]: remotia::pipeline::PipelineHandle

use std::{ffi::CString, sync::Arc};

use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParserContext};

use remotia::pipeline::PipelineHandle;
use tokio::sync::{mpsc, Mutex};

use crate::{builder::unwrap_mandatory, options::Options, scaling::Scaler};

mod puller;
mod pusher;

pub use puller::*;
pub use pusher::*;

/// Builder for constructing a ([`DecoderPusher`], [`DecoderPuller`]) pair.
///
/// # Mandatory fields
///
/// - [`codec_id`](DecoderBuilder::codec_id) — FFmpeg decoder name (e.g. `"h264"`, `"hevc"`)
/// - [`scaler`](DecoderBuilder::scaler) — a [`Scaler`] for pixel format conversion of decoded frames
///
/// # Optional fields
///
/// - [`options`](DecoderBuilder::options) — codec-specific options (defaults to empty)
/// - [`pipeline_handle`](DecoderBuilder::pipeline_handle) — for requesting pipeline
///   shutdown when the decoder finishes
pub struct DecoderBuilder {
    codec_id: Option<String>,
    options: Option<Options>,
    scaler: Option<Scaler>,
    pipeline_handle: Option<PipelineHandle>,
}

impl Default for DecoderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoderBuilder {
    /// Creates a new decoder builder with all fields unset.
    pub fn new() -> Self {
        Self {
            codec_id: None,
            options: None,
            scaler: None,
            pipeline_handle: None,
        }
    }

    builder_set!(options, Options);
    builder_set!(scaler, Scaler);

    /// Sets the FFmpeg decoder name (e.g. `"h264"`, `"hevc"`).
    pub fn codec_id(mut self, codec_id: &str) -> Self {
        self.codec_id = Some(codec_id.to_string());
        self
    }

    /// Sets the pipeline handle for requesting shutdown when the decoder finishes.
    ///
    /// When EOF is reached and the decoder is fully flushed, the pusher will call
    /// [`PipelineHandle::request_shutdown`] to gracefully stop the pipeline.
    pub fn pipeline_handle(mut self, handle: PipelineHandle) -> Self {
        self.pipeline_handle = Some(handle);
        self
    }

    /// Builds the decoder, returning a ([`DecoderPusher`], [`DecoderPuller`]) pair.
    ///
    /// Initializes the FFmpeg decoder, creates a codec parser context for splitting
    /// raw byte streams into packets, and sets up an unbounded channel for passing
    /// decoded frame data from the pusher to the puller.
    ///
    /// # Panics
    ///
    /// Panics if any mandatory field is missing, if the codec is not found, or if the
    /// codec context cannot be opened.
    pub fn build(self) -> (DecoderPusher, DecoderPuller) {
        let codec_id = unwrap_mandatory(self.codec_id);
        let options = self.options.unwrap_or_default().to_av_dict();

        let codec_id_string = CString::new(codec_id).unwrap();
        let decoder = AVCodec::find_decoder_by_name(&codec_id_string).unwrap();
        let parser_context = AVCodecParserContext::init(decoder.id).unwrap();

        let decode_context = {
            let mut decode_context = AVCodecContext::new(&decoder);
            decode_context.open(Some(options)).unwrap();

            Arc::new(Mutex::new(decode_context))
        };

        let scaler = unwrap_mandatory(self.scaler);

        let (frame_tx, frame_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        (
            DecoderPusher {
                decode_context: decode_context.clone(),
                parser_context,
                scaler,
                frame_tx: Some(frame_tx),
                pipeline_handle: self.pipeline_handle.clone(),
                eof_processed: false,
            },
            DecoderPuller {
                _decode_context: decode_context.clone(),
                frame_rx,
                pipeline_handle: self.pipeline_handle,
            },
        )
    }
}
