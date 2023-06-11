use clap::Parser;
use remotia::{
    buffers::BufferAllocator,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{component::Component, Pipeline},
    processors::ticker::Ticker, codecs::yuv::{yuv_to_rgba::YUV420PToRGBAConverter, rgba_to_yuv::RGBAToYUV420PConverter},
};

use crate::types::{FrameData, BufferType};

mod types;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 60)]
    framerate: u64
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
                .append(capturer)
                .append(RGBAToYUV420PConverter::new(
                    BufferType::RawFrameBuffer,
                    BufferType::YFrameBuffer,
                    BufferType::CBFrameBuffer,
                    BufferType::CRFrameBuffer,
                ))
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}

