use clap::Parser;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    capture::scrap::ScrapFrameCapturer,
    codecs::yuv::{rgba_to_yuv::RGBAToYUV420PConverter, yuv_to_rgba::YUV420PToRGBAConverter},
    pipeline::{component::Component, Pipeline},
    processors::ticker::Ticker,
    render::winit::WinitRenderer,
};
use remotia_ffmpeg_codecs::{
    decoders::h264::H264Decoder,
    encoders::{options::Options, x264::X264Encoder},
};

use crate::types::{BufferType::*, FrameData};

mod types;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 60)]
    framerate: u64,
}

const POOLS_SIZE: usize = 1;

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World! I will mirror your screen encoding it using the H264 codec.");

    let args = Args::parse();

    let capturer = ScrapFrameCapturer::new_from_primary(CapturedRGBAFrameBuffer);

    log::info!("Streaming at {}x{}", capturer.width(), capturer.height());

    let width = capturer.width() as usize;
    let height = capturer.height() as usize;
    let pixels_count = width * height;
    let mut registry = PoolRegistry::new();
    registry.register(CapturedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4).await;
    registry.register(EncodedFrameBuffer, POOLS_SIZE, pixels_count).await;
    registry.register(DecodedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4).await;

    let encoder = X264Encoder::new(
        width as i32,
        height as i32,
        CapturedRGBAFrameBuffer,
        EncodedFrameBuffer,
        Options::new().set("crf", "26").set("tune", "zerolatency"),
    );

    let decoder = H264Decoder::new(EncodedFrameBuffer, DecodedYBuffer, DecodedCBBuffer, DecodedCRBuffer);

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000 / args.framerate))
                .append(registry.get(CapturedRGBAFrameBuffer).borrower())
                .append(capturer)
                // .append(registry.get(YBuffer).borrower())
                // .append(registry.get(CBBuffer).borrower())
                // .append(registry.get(CRBuffer).borrower())
                // .append(RGBAToYUV420PConverter::new(width, CapturedRGBAFrameBuffer, YBuffer, CBBuffer, CRBuffer))
                // .append(registry.get(CapturedRGBAFrameBuffer).redeemer())

                .append(encoder.pusher())
                .append(registry.get(CapturedRGBAFrameBuffer).redeemer())
                // .append(registry.get(YBuffer).redeemer())
                // .append(registry.get(CBBuffer).redeemer())
                // .append(registry.get(CRBuffer).redeemer())
                .append(registry.get(EncodedFrameBuffer).borrower())
                .append(encoder.puller())
                // .append(registry.get(DecodedYBuffer).borrower())
                // .append(registry.get(DecodedCBBuffer).borrower())
                // .append(registry.get(DecodedCRBuffer).borrower())
                // .append(decoder)
                .append(registry.get(EncodedFrameBuffer).redeemer())
                // .append(registry.get(DecodedRGBAFrameBuffer).borrower())
                // .append(YUV420PToRGBAConverter::new(DecodedYBuffer, DecodedCBBuffer, DecodedCRBuffer, DecodedRGBAFrameBuffer))

                // .append(registry.get(DecodedRGBAFrameBuffer).borrower())
                // .append(YUV420PToRGBAConverter::new(width, YBuffer, CBBuffer, CRBuffer, DecodedRGBAFrameBuffer))
                // .append(registry.get(YBuffer).redeemer())
                // .append(registry.get(CBBuffer).redeemer())
                // .append(registry.get(CRBuffer).redeemer())

                // .append(registry.get(DecodedYBuffer).redeemer())
                // .append(registry.get(DecodedCBBuffer).redeemer())
                // .append(registry.get(DecodedCRBuffer).redeemer())
                // .append(WinitRenderer::new(DecodedRGBAFrameBuffer, width as u32, height as u32))
                // .append(registry.get(DecodedRGBAFrameBuffer).redeemer()),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
