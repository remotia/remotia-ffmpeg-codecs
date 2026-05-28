//! # remotia-ffmpeg-codecs
//!
//! FFmpeg-based encoding and decoding codecs for [remotia] pipelines, built on top of
//! [rsmpeg].
//!
//! [remotia]: https://github.com/remotia/remotia
//! [rsmpeg]: https://github.com/larksuite/rsmpeg
//!
//! This crate provides a pusher/puller architecture that integrates FFmpeg codecs into
//! remotia's [`FrameProcessor`] pipeline model. Encoding and decoding are each split into
//! two cooperating processors that run in separate pipeline components connected by the
//! pipeline's frame-passing mechanism, plus an internal notification channel for
//! back-pressure:
//!
//! - **Encoders**: [`EncoderPusher`] sends raw frames into the FFmpeg encoder; [`EncoderPuller`]
//!   receives encoded packets back.
//! - **Decoders**: [`DecoderPusher`] feeds encoded packets into the FFmpeg decoder; [`DecoderPuller`]
//!   receives decoded frames back.
//!
//! Both pusher/puller pairs are created together by their respective builders and share an
//! `Arc<Mutex<AVCodecContext>>` internally, so they must be placed in **separate pipeline
//! components** linked together (i.e. the pusher in one component, the puller in the next).
//!
//! # Quick start
//!
//! ## Encoding pipeline
//!
//! ```text
//! [Capturer → EncoderPusher] → [EncoderPuller → PacketWriter]
//!       component 1                  component 2
//! ```
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
//! ## Decoding pipeline
//!
//! ```text
//! [ChunkReader → DecoderPusher] → [DecoderPuller → FrameWriter]
//!       component 1                      component 2
//! ```
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
//! # Implementing `FFMpegCodec`
//!
//! Frame data types flowing through the pipeline must implement the [`FFMpegCodec`] trait,
//! which provides the bridge between the generic frame data and FFmpeg's packet/frame
//! buffers. See the trait documentation for details.
//!
//! # Module overview
//!
//! - [`encoders`] — Encoder pusher/puller processors and builder.
//! - [`decoders`] — Decoder pusher/puller processors and builder.
//! - [`scaling`] — Pixel format conversion and scaling via FFmpeg's `swscale`.
//! - [`options`] — Codec options as key/value pairs, convertible to `AVDictionary`.
//! - [`ffi`] — Re-export of `rsmpeg::ffi` for FFmpeg pixel format constants, etc.

#[macro_use]
mod builder;

pub mod decoders;
pub mod encoders;
pub mod scaling;
pub mod options;

pub use rsmpeg::ffi;

/// Trait that bridges frame data with FFmpeg codec operations.
///
/// Any frame data type flowing through an encoding or decoding pipeline must implement
/// this trait. It defines how encoded packet data is written to and read from the frame,
/// how decoded pixel buffers are populated, how codec errors are reported, and how frame
/// identifiers (PTS) are managed.
///
/// # Typical implementation
///
/// The implementor usually maps these methods to typed buffer slots and stat keys within
/// a [`BuffersMap`]:
///
/// - [`write_packet_data`](FFMpegCodec::write_packet_data) and
///   [`get_packet_data_buffer`](FFMpegCodec::get_packet_data_buffer) manage an
///   "encoded packet" buffer slot.
/// - [`write_decoded_buffer`](FFMpegCodec::write_decoded_buffer) manages a "decoded frame"
///   buffer slot.
/// - [`report_flush_error`](FFMpegCodec::report_flush_error),
///   [`report_codec_error`](FFMpegCodec::report_codec_error), and
///   [`report_decoder_drain_error`](FFMpegCodec::report_decoder_drain_error) report
///   error states through the frame's error slot.
/// - [`set_frame_id`](FFMpegCodec::set_frame_id) and
///   [`get_frame_id`](FFMpegCodec::get_frame_id) track the presentation timestamp.
/// - [`is_eof`](FFMpegCodec::is_eof) signals end-of-stream (defaults to `false`).
///
/// [`BuffersMap`]: remotia::buffers::BuffersMap
pub trait FFMpegCodec {
    /// Appends encoded packet data to the frame's packet buffer.
    fn write_packet_data(&mut self, packet_data: &[u8]);

    /// Returns the current encoded packet data as a byte slice.
    fn get_packet_data_buffer(&self) -> &[u8];

    /// Appends decoded pixel data to the frame's decoded buffer.
    fn write_decoded_buffer(&mut self, data: &[u8]);

    /// Reports that the encoder has been fully flushed (no more packets will be produced).
    fn report_flush_error(&mut self);

    /// Reports a general codec error during encoding or decoding.
    fn report_codec_error(&mut self);

    /// Reports that the decoder has drained all available frames (temporary condition).
    fn report_decoder_drain_error(&mut self);

    /// Sets the frame identifier (typically the presentation timestamp / PTS).
    fn set_frame_id(&mut self, frame_id: i64);

    /// Returns the frame identifier (typically the presentation timestamp / PTS).
    fn get_frame_id(&self) -> i64;

    /// Returns `true` if this frame signals end-of-stream.
    ///
    /// The default implementation returns `false`. Override this to detect EOF frames
    /// that trigger encoder/decoder flushing.
    fn is_eof(&self) -> bool {
        false
    }
}
