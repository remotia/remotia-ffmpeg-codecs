use clap::Parser;
use remotia::{
    buffers::BufferAllocator,
    capture::scrap::ScrapFrameCapturer,
    codecs::yuv::{rgba_to_yuv::RGBAToYUV420PConverter, yuv_to_rgba::YUV420PToRGBAConverter},
    pipeline::{component::Component, Pipeline},
    processors::ticker::Ticker,
    render::winit::WinitRenderer,
};
use remotia_ffmpeg_codecs::{encoders::{options::Options, x264::X264Encoder}, decoders::h264::H264Decoder};

use crate::types::{BufferType, FrameData};

mod types;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 60)]
    framerate: u64,

    #[arg(long)]
    width: u32,

    #[arg(long)]
    height: u32,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World! I will mirror your screen encoding it using the H264 codec.");

    let args = Args::parse();

    let mut capturer = ScrapFrameCapturer::new_from_primary(BufferType::CapturedRGBAFrameBuffer);

    log::info!("Streaming at {}x{}", capturer.width(), capturer.height());

    let encoder = X264Encoder::new(
        args.width as i32,
        args.height as i32,
        BufferType::YBuffer,
        BufferType::CBBuffer,
        BufferType::CRBuffer,
        BufferType::EncodedFrameBuffer,
        Options::new()
            .set("crf", "26")
            .set("tune", "zerolatency"),
    );

    let decoder = H264Decoder::new(
        BufferType::EncodedFrameBuffer,
        BufferType::DecodedYBuffer,
        BufferType::DecodedCBBuffer,
        BufferType::DecodedCRBuffer,
    );

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000 / args.framerate))
                .append(BufferAllocator::new(
                    BufferType::CapturedRGBAFrameBuffer,
                    capturer.buffer_size(),
                ))
                .append(capturer)
                .append(BufferAllocator::new(
                    BufferType::YBuffer,
                    (args.width * args.height) as usize,
                ))
                .append(BufferAllocator::new(
                    BufferType::CBBuffer,
                    ((args.width * args.height) / 4) as usize,
                ))
                .append(BufferAllocator::new(
                    BufferType::CRBuffer,
                    ((args.width * args.height) / 4) as usize,
                ))
                .append(RGBAToYUV420PConverter::new(
                    BufferType::CapturedRGBAFrameBuffer,
                    BufferType::YBuffer,
                    BufferType::CBBuffer,
                    BufferType::CRBuffer,
                ))
                .append(encoder.pusher())
                .append(BufferAllocator::new(
                    BufferType::EncodedFrameBuffer,
                    (args.width * args.height * 4) as usize,
                ))
                .append(encoder.puller())
                .append(BufferAllocator::new(
                    BufferType::DecodedYBuffer,
                    (args.width * args.height) as usize,
                ))
                .append(BufferAllocator::new(
                    BufferType::DecodedCBBuffer,
                    ((args.width * args.height) / 4) as usize,
                ))
                .append(BufferAllocator::new(
                    BufferType::DecodedCRBuffer,
                    ((args.width * args.height) / 4) as usize,
                ))
                .append(decoder)
                .append(BufferAllocator::new(
                    BufferType::DecodedRGBAFrameBuffer,
                    (args.width * args.height * 4) as usize,
                ))
                .append(YUV420PToRGBAConverter::new(
                    BufferType::DecodedYBuffer,
                    BufferType::DecodedCBBuffer,
                    BufferType::DecodedCRBuffer,
                    BufferType::DecodedRGBAFrameBuffer,
                ))
                .append(WinitRenderer::new(
                    BufferType::DecodedRGBAFrameBuffer,
                    args.width,
                    args.height,
                )),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
