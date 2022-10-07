use log::debug;
use remotia::types::FrameData;
use rsmpeg::avcodec::AVCodecContext;

use crate::encoders::utils::packet::receive_encoded_packet;

pub fn pull_packet(encode_context: &mut AVCodecContext, frame_data: &mut FrameData) {
    let mut output_buffer = frame_data
        .extract_writable_buffer("encoded_frame_buffer")
        .expect("No encoded frame buffer in frame DTO");

    let encoded_bytes = receive_encoded_packet(encode_context, &mut output_buffer);

    debug!(
        "Pulled encoded packet for frame {} (size = {})",
        frame_data.get("capture_timestamp"),
        encoded_bytes
    );

    frame_data.insert_writable_buffer("encoded_frame_buffer", output_buffer);

    frame_data.set("encoded_size", encoded_bytes as u128);
}
