use rsmpeg::{avcodec::AVCodecContext, avutil::AVFrame};

pub fn send_avframe(encode_context: &mut AVCodecContext, avframe: AVFrame) {
    encode_context.send_frame(Some(&avframe)).unwrap();
}
