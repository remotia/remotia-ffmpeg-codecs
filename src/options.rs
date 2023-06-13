use std::{collections::HashMap, ffi::CString};

use rsmpeg::avutil::AVDictionary;

use cstr::cstr;

#[derive(Default, Clone)]
pub struct Options {
    pairs: HashMap<String, (String, u32)>,
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, key: &str, value: &str) -> Self {
        self.pairs.insert(key.to_string(), (value.to_string(), 0));
        self
    }

    pub fn set_flags(mut self, key: &str, value: &str, flags: u32) -> Self {
        self.pairs
            .insert(key.to_string(), (value.to_string(), flags));
        self
    }

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
