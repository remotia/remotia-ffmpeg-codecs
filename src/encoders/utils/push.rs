use remotia::types::FrameData;
use rsmpeg::avcodec::AVCodecContext;

use crate::encoders::utils::frame_builders::yuv420p::YUV420PAVFrameBuilder;

use super::avframe::send_avframe;

pub fn push_frame(
    encode_context: &mut AVCodecContext,
    avframe_builder: &mut YUV420PAVFrameBuilder,
    frame_data: &mut FrameData,
) {
    let y_channel_buffer = frame_data
        .extract_writable_buffer("y_channel_buffer")
        .unwrap();

    let cb_channel_buffer = frame_data
        .extract_writable_buffer("cb_channel_buffer")
        .unwrap();

    let cr_channel_buffer = frame_data
        .extract_writable_buffer("cr_channel_buffer")
        .unwrap();

    let avframe = avframe_builder.create_avframe(
        encode_context,
        frame_data.get("capture_timestamp") as i64,
        &y_channel_buffer,
        &cb_channel_buffer,
        &cr_channel_buffer,
        false,
    );

    send_avframe(encode_context, avframe);

    frame_data.insert_writable_buffer("y_channel_buffer", y_channel_buffer);
    frame_data.insert_writable_buffer("cb_channel_buffer", cb_channel_buffer);
    frame_data.insert_writable_buffer("cr_channel_buffer", cr_channel_buffer);
}
