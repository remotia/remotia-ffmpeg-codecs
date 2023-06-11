use std::collections::HashMap;

use bytes::BytesMut;
use remotia::traits::BorrowableFrameProperties;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    RawFrameBuffer,
    YFrameBuffer,
    CBFrameBuffer,
    CRFrameBuffer,
    EncodedFrameBuffer,
}

#[derive(Default, Debug)]
pub struct FrameData {
    buffers: HashMap<BufferType, BytesMut>
}

impl BorrowableFrameProperties<BufferType, BytesMut> for FrameData {
    fn push(&mut self, key: BufferType, value: BytesMut) {
        self.buffers.insert(key, value);
    }

    fn pull(&mut self, key: &BufferType) -> Option<BytesMut> {
        self.buffers.remove(key)
    }

    fn get_ref(&self, key: &BufferType) -> Option<&BytesMut> {
        self.buffers.get(key)
    }

    fn get_mut_ref(&mut self, key: &BufferType) -> Option<&mut BytesMut> {
        self.buffers.get_mut(key)
    }
}
