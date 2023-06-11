use rsmpeg::avcodec::AVCodecContext;

use crate::encoders::utils::frame_builders::yuv420p::YUV420PAVFrameBuilder;

use super::avframe::send_avframe;

pub fn push_frame(
    encode_context: &mut AVCodecContext,
    avframe_builder: &mut YUV420PAVFrameBuilder,
    timestamp: i64,
    y_channel_buffer: &[u8],
    cb_channel_buffer: &[u8],
    cr_channel_buffer: &[u8]
) {
    let avframe = avframe_builder.create_avframe(
        encode_context,
        timestamp,
        &y_channel_buffer,
        &cb_channel_buffer,
        &cr_channel_buffer,
        false,
    );

    send_avframe(encode_context, avframe);
}
