use clap::Parser;
use remotia::{
    buffers::BufferAllocator,
    capture::scrap::ScrapFrameCapturer,
    codecs::yuv::{rgba_to_yuv::RGBAToYUV420PConverter, yuv_to_rgba::YUV420PToRGBAConverter},
    pipeline::{component::Component, Pipeline},
    processors::ticker::Ticker,
    render::winit::WinitRenderer,
};

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

    let mut capturer = ScrapFrameCapturer::new_from_primary(BufferType::RawFrameBuffer);

    log::info!("Streaming at {}x{}", capturer.width(), capturer.height());

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000 / args.framerate))
                .append(BufferAllocator::new(
                    BufferType::RawFrameBuffer,
                    capturer.buffer_size(),
                ))
                .append(BufferAllocator::new(
                    BufferType::YFrameBuffer,
                    (args.width * args.height) as usize,
                ))
                .append(BufferAllocator::new(
                    BufferType::CBFrameBuffer,
                    ((args.width * args.height) / 4) as usize,
                ))
                .append(BufferAllocator::new(
                    BufferType::CRFrameBuffer,
                    ((args.width * args.height) / 4) as usize,
                ))
                .append(capturer)
                .append(RGBAToYUV420PConverter::new(
                    BufferType::RawFrameBuffer,
                    BufferType::YFrameBuffer,
                    BufferType::CBFrameBuffer,
                    BufferType::CRFrameBuffer,
                ))
                .append(YUV420PToRGBAConverter::new(
                    BufferType::YFrameBuffer,
                    BufferType::CBFrameBuffer,
                    BufferType::CRFrameBuffer,
                    BufferType::RawFrameBuffer,
                ))
                .append(WinitRenderer::new(
                    BufferType::RawFrameBuffer,
                    args.width,
                    args.height,
                )),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
