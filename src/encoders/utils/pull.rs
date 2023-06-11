use rsmpeg::avcodec::AVCodecContext;

use crate::encoders::utils::packet::receive_encoded_packet;

pub fn pull_packet(encode_context: &mut AVCodecContext, output_buffer: &mut [u8]) {
    let encoded_bytes = receive_encoded_packet(encode_context, output_buffer);
}
