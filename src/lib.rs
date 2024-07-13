#[macro_use]
mod builder;

pub mod decoders;
pub mod encoders;
pub mod scaling;
pub mod options;

pub use rsmpeg::ffi;

pub trait FFMpegCodec {
    fn write_packet_data(&mut self, packet_data: &[u8]);
    fn get_packet_data_buffer(&self) -> &[u8];
    fn write_decoded_buffer(&mut self, data: &[u8]);
    fn report_flush_error(&mut self);
    fn report_codec_error(&mut self);
    fn report_decoder_drain_error(&mut self);
    fn set_frame_id(&mut self, frame_id: i64);
    fn get_frame_id(&self) -> i64;
}
