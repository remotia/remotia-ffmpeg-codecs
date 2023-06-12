use std::io::Write;

use bytes::BufMut;
use log::debug;
use remotia::buffers::BufferMut;
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

pub fn receive_encoded_packet(
    encode_context: &mut AVCodecContext,
    output_buffer: &mut BufferMut,
) {
    loop {
        let packet = match encode_context.receive_packet() {
            Ok(packet) => {
                // debug!("Received packet of size {}", packet.size);
                packet
            }
            Err(RsmpegError::EncoderDrainError) => {
                debug!("Drain error, breaking the loop");
                break;
            }
            Err(RsmpegError::EncoderFlushedError) => {
                debug!("Flushed error, breaking the loop");
                break;
            }
            Err(e) => panic!("{:?}", e),
        };

        let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };

        debug!("Encoded packet: {:?}", packet);

        output_buffer.put(data);
    }
}
