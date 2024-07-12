use log::{debug, trace};
use rsmpeg::{
    avcodec::{AVCodecContext, AVCodecParserContext, AVPacket},
    UnsafeDerefMut,
};

pub fn parse_and_send_packets(
    decode_context: &mut AVCodecContext,
    parser_context: &mut AVCodecParserContext,
    input_buffer: &[u8],
    frame_id: i64,
) -> Result<(), ()> {
    let mut packet = AVPacket::new();
    let mut parsed_offset = 0;

    packet.set_pts(frame_id);

    unsafe {
        let raw = decode_context.as_ptr();
        trace!("Raw context: {:#?}", *raw);
    }

    debug!(
        "Parsing packets (timestamp: {}, input buffer size: {})...",
        frame_id,
        input_buffer.len()
    );

    while parsed_offset < input_buffer.len() {
        let (get_packet, offset) = {
            let this = &mut *parser_context;
            let packet: &mut AVPacket = &mut packet;
            let mut packet_data = packet.data;
            let mut packet_size = packet.size;
            
            let mut offset = 0;
            loop {
                let current_offset = unsafe {
                    rsmpeg::ffi::av_parser_parse2(
                        this.as_mut_ptr(),
                        decode_context.as_mut_ptr(),
                        &mut packet_data,
                        &mut packet_size,
                        input_buffer.as_ptr(),
                        input_buffer.len() as i32,
                        packet.pts,
                        packet.dts,
                        packet.pos,
                    )
                };

                log::trace!("Parsing current packet size: {}", packet.size);
                log::trace!("Parsing current offset: {}", current_offset);

                offset += current_offset;

                if packet_size > 0 {
                    break;
                }
            };

            unsafe {
                packet.deref_mut().data = packet_data;
                packet.deref_mut().size = packet_size;
            }

            log::trace!("Parsing final packet size: {}", packet.size);
            log::trace!("Parsing final offset: {}", offset);

            (packet.size != 0, offset as usize)
        };

        if get_packet {
            unsafe {
                let raw = packet.as_ptr();
                trace!("Decoded raw packet: {:#?}", *raw);
            }

            let result = decode_context.send_packet(Some(&packet));

            match result {
                Ok(_) => {
                    trace!("Sent packet successfully");
                }
                Err(e) => {
                    debug!("Error on send packet: {}", e);
                    return Err(());
                }
            }

            packet = AVPacket::new();
        } else {
            debug!("No more packets to be sent");
        }

        parsed_offset += offset;
    }

    Ok(())
}
