use std::collections::HashMap;

use remotia::{
    buffers::{BufMut, BytesMut},
    traits::{BorrowFrameProperties, BorrowMutFrameProperties, FrameError, PullableFrameProperties},
};
use remotia_ffmpeg_codecs::FFMpegCodec;

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
    FlushError,
}

#[derive(Default, Debug)]
pub struct FrameData {
    frame_id: i64,
    buffers: HashMap<BufferType, BytesMut>,
    error: Option<Error>,
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

impl FFMpegCodec for FrameData {
    fn write_packet_data(&mut self, packet_data: &[u8]) {
        self.get_mut_ref(&BufferType::EncodedFrameBuffer)
            .unwrap()
            .put(packet_data)
    }

    fn get_packet_data_buffer(&self) -> &[u8] {
        self.get_ref(&BufferType::EncodedFrameBuffer).unwrap()
    }

    fn write_decoded_buffer(&mut self, data: &[u8]) {
        self.get_mut_ref(&BufferType::DecodedRGBAFrameBuffer)
            .unwrap()
            .put(data)
    }

    fn report_flush_error(&mut self) {
        self.report_error(Error::FlushError)
    }

    fn report_codec_error(&mut self) {
        self.report_error(Error::CodecError)
    }

    fn report_decoder_drain_error(&mut self) {
        self.report_error(Error::NoFrame)
    }

    fn set_frame_id(&mut self, frame_id: i64) {
        self.frame_id = frame_id;
    }

    fn get_frame_id(&self) -> i64 {
        self.frame_id
    }
}
