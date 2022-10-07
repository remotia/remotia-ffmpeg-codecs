use log::debug;
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

pub fn receive_encoded_packet(
    encode_context: &mut AVCodecContext,
    output_buffer: &mut [u8],
) -> usize {
    let mut encoded_frame_length = 0;

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

        let start_index = encoded_frame_length;
        let end_index = encoded_frame_length + data.len();

        output_buffer[start_index..end_index].copy_from_slice(data);

        encoded_frame_length = end_index;
    }

    encoded_frame_length
}
