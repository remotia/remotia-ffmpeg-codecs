use clap::{Parser, error};
use remotia::{
    buffers::pool_registry::PoolRegistry,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{component::Component, Pipeline},
    processors::{ticker::Ticker, error_switch::OnErrorSwitch, functional::Function},
    render::winit::WinitRenderer,
};
use remotia_ffmpeg_codecs::{
    decoders::h264::H264Decoder,
    encoders::{options::Options, x264::X264Encoder},
};

use crate::types::{BufferType::*, Error::*, FrameData};

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
    registry
        .register(CapturedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;
    registry
        .register(EncodedFrameBuffer, POOLS_SIZE, pixels_count)
        .await;
    registry
        .register(DecodedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;

    let encoder = X264Encoder::new(
        width as i32,
        height as i32,
        CapturedRGBAFrameBuffer,
        EncodedFrameBuffer,
        Options::new().set("crf", "26").set("tune", "zerolatency"),
    );

    let decoder = H264Decoder::new(
        width as i32,
        height as i32,
        EncodedFrameBuffer,
        DecodedRGBAFrameBuffer,
        NoFrame,
        CodecError,
    );

    let mut error_pipeline = Pipeline::<FrameData>::singleton(
        Component::new()
            .append(Function::new(|fd| {
                log::warn!("Dropped frame");
                Some(fd)
            }))
            .append(registry.get(CapturedRGBAFrameBuffer).redeemer().soft())
            .append(registry.get(EncodedFrameBuffer).redeemer().soft())
            .append(registry.get(DecodedRGBAFrameBuffer).redeemer().soft())
    ).feedable();

    let pipeline = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000 / args.framerate))
                .append(registry.get(CapturedRGBAFrameBuffer).borrower())
                .append(capturer)
                .append(encoder.pusher())
                .append(registry.get(CapturedRGBAFrameBuffer).redeemer())
                .append(registry.get(EncodedFrameBuffer).borrower())
                .append(encoder.puller())
                .append(registry.get(DecodedRGBAFrameBuffer).borrower())
                .append(decoder)
                .append(registry.get(EncodedFrameBuffer).redeemer())
                .append(OnErrorSwitch::new(&mut error_pipeline))
                .append(WinitRenderer::new(DecodedRGBAFrameBuffer, width as u32, height as u32))
                .append(registry.get(DecodedRGBAFrameBuffer).redeemer()),
        );

    let mut handles = Vec::new();
    handles.extend(error_pipeline.run());
    handles.extend(pipeline.run());

    for handle in handles {
        handle.await.unwrap();
    }
}
