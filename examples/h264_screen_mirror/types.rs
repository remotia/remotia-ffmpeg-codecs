use std::collections::HashMap;

use bytes::BytesMut;
use remotia::traits::{BorrowableFrameProperties, PullableFrameProperties, BorrowFrameProperties, BorrowMutFrameProperties};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    CapturedRGBAFrameBuffer,
    YFrameBuffer,
    CBFrameBuffer,
    CRFrameBuffer,
    EncodedFrameBuffer,
    DecodedRGBAFrameBuffer,
}

#[derive(Default, Debug)]
pub struct FrameData {
    buffers: HashMap<BufferType, BytesMut>,
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

