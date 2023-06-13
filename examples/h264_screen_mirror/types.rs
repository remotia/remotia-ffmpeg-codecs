use std::collections::HashMap;

use remotia::{traits::{PullableFrameProperties, BorrowFrameProperties, BorrowMutFrameProperties, FrameError}, buffers::BytesMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    CapturedRGBAFrameBuffer,
    EncodedFrameBuffer,
    DecodedRGBAFrameBuffer,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    NoFrame,
    CodecError,
}

#[derive(Default, Debug)]
pub struct FrameData {
    buffers: HashMap<BufferType, BytesMut>,
    error: Option<Error>
}

impl PullableFrameProperties<BufferType, BytesMut> for FrameData {
    fn push(&mut self, key: BufferType, value: BytesMut) {
        self.buffers.insert(key, value);
    }

    fn pull(&mut self, key: &BufferType) -> Option<BytesMut> {
        self.buffers.remove(key)
    }
}

impl BorrowFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_ref(&self, key: &BufferType) -> Option<&BytesMut> {
        self.buffers.get(key)
    }
}

impl BorrowMutFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_mut_ref(&mut self, key: &BufferType) -> Option<&mut BytesMut> {
        self.buffers.get_mut(key)
    }
}

impl FrameError<Error> for FrameData {
    fn report_error(&mut self, error: Error) {
        self.error = Some(error);
    }

    fn get_error(&self) -> Option<Error> {
        self.error
    }
}

