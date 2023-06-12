use log::{debug, trace};
use rsmpeg::{
    avcodec::{AVCodecContext, AVCodecParserContext},
    error::RsmpegError,
};

use crate::decoders::utils::avframe::write_avframe_to_yuv;

use self::packet::parse_packets;

pub mod avframe;
pub mod packet;
#[allow(dead_code)]
pub mod yuv2bgr;

pub fn decode_to_yuv(
    decode_context: &mut AVCodecContext,
    parser_context: &mut AVCodecParserContext,
    timestamp: i64,
    encoded_buffer: &[u8],
    y_buffer: &mut [u8],
    cb_buffer: &mut [u8],
    cr_buffer: &mut [u8]
) -> Result<(), ()> {
    let decode_result = if let Some(error) = parse_packets(
        decode_context,
        parser_context,
        encoded_buffer,
        timestamp,
    ) {
        Err(error)
    } else {
        loop {
            match decode_context.receive_frame() {
                Ok(avframe) => {
                    trace!("Received AVFrame: {:?}", avframe);

                    write_avframe_to_yuv(
                        avframe,
                        y_buffer,
                        cb_buffer,
                        cr_buffer,
                    );

                    // Override capture timestamp to compensate any codec delay
                    // let received_capture_timestamp = parser_context.last_pts as u128;
                    // frame_data.set("capture_timestamp", received_capture_timestamp);

                    break Ok(());
                }
                Err(RsmpegError::DecoderDrainError) => {
                    debug!("No frames to be pulled");
                    break Err(());
                }
                Err(RsmpegError::DecoderFlushedError) => {
                    panic!("Decoder has been flushed unexpectedly");
                }
                Err(e) => panic!("{:?}", e),
            }
        }
    };

    decode_result
}
