use log::{debug, trace};
use remotia::{error::DropReason, types::FrameData};
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
    frame_data: &mut FrameData,
) -> Result<(), DropReason> {
    let mut encoded_frame_buffer = frame_data
        .extract_writable_buffer("encoded_frame_buffer")
        .unwrap();

    let empty_buffer_memory =
        encoded_frame_buffer.split_off(frame_data.get("encoded_size") as usize);

    let mut y_channel_buffer = frame_data
        .extract_writable_buffer("y_channel_buffer")
        .unwrap();

    let mut cb_channel_buffer = frame_data
        .extract_writable_buffer("cb_channel_buffer")
        .unwrap();

    let mut cr_channel_buffer = frame_data
        .extract_writable_buffer("cr_channel_buffer")
        .unwrap();

    let capture_timestamp = frame_data.get("capture_timestamp");

    let pts = capture_timestamp as i64;
    let decode_result = if let Some(error) = parse_packets(
        decode_context,
        parser_context,
        &mut encoded_frame_buffer,
        pts,
    ) {
        Err(error)
    } else {
        loop {
            match decode_context.receive_frame() {
                Ok(avframe) => {
                    trace!("Received AVFrame: {:?}", avframe);

                    write_avframe_to_yuv(
                        avframe,
                        &mut y_channel_buffer,
                        &mut cb_channel_buffer,
                        &mut cr_channel_buffer,
                    );

                    // Override capture timestamp to compensate any codec delay
                    let received_capture_timestamp = parser_context.last_pts as u128;
                    frame_data.set("capture_timestamp", received_capture_timestamp);

                    break Ok(());
                }
                Err(RsmpegError::DecoderDrainError) => {
                    debug!("No frames to be pulled");
                    break Err(DropReason::NoDecodedFrames);
                }
                Err(RsmpegError::DecoderFlushedError) => {
                    panic!("Decoder has been flushed unexpectedly");
                }
                Err(e) => panic!("{:?}", e),
            }
        }
    };

    encoded_frame_buffer.unsplit(empty_buffer_memory);

    frame_data.insert_writable_buffer("encoded_frame_buffer", encoded_frame_buffer);
    frame_data.insert_writable_buffer("y_channel_buffer", y_channel_buffer);
    frame_data.insert_writable_buffer("cb_channel_buffer", cb_channel_buffer);
    frame_data.insert_writable_buffer("cr_channel_buffer", cr_channel_buffer);

    decode_result
}
