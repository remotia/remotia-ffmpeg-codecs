//! Codec options as key/value pairs, convertible to FFmpeg's [`AVDictionary`].
//!
//! Use [`Options`] to configure encoder or decoder parameters before building a codec.
//! Options are set in a builder pattern and converted to an [`AVDictionary`] when the
//! codec context is opened.
//!
//! # Example
//!
//! ```no_run
//! use remotia_ffmpeg_codecs::options::Options;
//!
//! let options = Options::new()
//!     .set("crf", "26")
//!     .set("tune", "zerolatency")
//!     .set("preset", "ultrafast");
//! ```

use std::{collections::HashMap, ffi::CString};

use rsmpeg::avutil::AVDictionary;

use cstr::cstr;

/// A collection of FFmpeg codec options as key/value pairs with optional flags.
///
/// Each entry stores a key, a value, and a `u32` flags field that is passed directly
/// to [`AVDictionary::set`]. The [`set`](Options::set) method uses flags `0` (the common
/// case), while [`set_flags`](Options::set_flags) allows specifying custom flags.
///
/// Convert to an [`AVDictionary`] with [`to_av_dict`](Options::to_av_dict).
#[derive(Default, Clone)]
pub struct Options {
    pairs: HashMap<String, (String, u32)>,
}

impl Options {
    /// Creates an empty options set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a key/value pair with default flags (`0`).
    ///
    /// This is the most common way to set codec options such as `"crf"`, `"tune"`,
    /// `"preset"`, etc.
    pub fn set(mut self, key: &str, value: &str) -> Self {
        self.pairs.insert(key.to_string(), (value.to_string(), 0));
        self
    }

    /// Inserts a key/value pair with explicit flags.
    ///
    /// The `flags` parameter is passed through to FFmpeg's `av_dict_set` when
    /// [`to_av_dict`](Options::to_av_dict) is called.
    pub fn set_flags(mut self, key: &str, value: &str, flags: u32) -> Self {
        self.pairs
            .insert(key.to_string(), (value.to_string(), flags));
        self
    }

    /// Converts the options into an FFmpeg [`AVDictionary`].
    ///
    /// This is called internally by [`EncoderBuilder::build`](crate::encoders::EncoderBuilder::build)
    /// and [`DecoderBuilder::build`](crate::decoders::DecoderBuilder::build) when opening
    /// the codec context.
    pub fn to_av_dict(self) -> AVDictionary {
        let mut dict = AVDictionary::new(cstr!(""), cstr!(""), 0);

        for (key, (value, flags)) in self.pairs {
            dict = dict.set(
                &CString::new(key).unwrap(),
                &CString::new(value).unwrap(),
                flags,
            );
        }

        dict
    }
}
