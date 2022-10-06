use log::{debug, trace};
use remotia::error::DropReason;
use rsmpeg::{avcodec::{AVCodecContext, AVPacket, AVCodecParserContext}, ffi, UnsafeDerefMut};

pub fn parse_packets(
    mut decode_context: &mut AVCodecContext,
    mut parser_context: &mut AVCodecParserContext,
    input_buffer: &[u8],
    timestamp: i64,
) -> Option<DropReason> {
    let mut packet = AVPacket::new();
    let mut parsed_offset = 0;

    debug!(
        "Parsing packets (timestamp: {}, input buffer size: {})...",
        timestamp,
        input_buffer.len()
    );

    while parsed_offset < input_buffer.len() {
        let (get_packet, offset) = {
            let this = &mut parser_context;

            let codec_context: &mut AVCodecContext = &mut decode_context;
            let packet: &mut AVPacket = &mut packet;
            let data: &[u8] = &input_buffer[parsed_offset..];
            let mut packet_data = packet.data;
            let mut packet_size = packet.size;

            let offset = unsafe {
                ffi::av_parser_parse2(
                    this.as_mut_ptr(),
                    codec_context.as_mut_ptr(),
                    &mut packet_data,
                    &mut packet_size,
                    data.as_ptr(),
                    data.len() as i32,
                    timestamp,
                    timestamp,
                    0,
                )
            };

            unsafe {
                packet.deref_mut().data = packet_data;
                packet.deref_mut().size = packet_size;
            }

            (packet.size != 0, offset as usize)
        };

        if get_packet {
            let result = decode_context.send_packet(Some(&packet));

            match result {
                Ok(_) => {
                    trace!("Sent packet successfully");
                }
                Err(e) => {
                    debug!("Error on send packet: {}", e);
                    return Some(DropReason::CodecError);
                }
            }

            trace!("Decoded packet: {:?}", packet);

            packet = AVPacket::new();
        } else {
            debug!("No more packets to be sent");
        }

        parsed_offset += offset;
    }

    None
}
