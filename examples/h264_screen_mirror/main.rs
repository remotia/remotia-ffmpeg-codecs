use clap::Parser;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{component::Component, Pipeline},
    processors::{error_switch::OnErrorSwitch, functional::Function, ticker::Ticker},
    render::winit::WinitRenderer,
};
use remotia_ffmpeg_codecs::{
    ffi, encoders::EncoderBuilder, options::Options, decoders::DecoderBuilder, scaling::ScalerBuilder,
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

    let (encoder_pusher, encoder_puller) = EncoderBuilder::new()
        .codec_id("libx264")
        .rgba_buffer_key(CapturedRGBAFrameBuffer)
        .encoded_buffer_key(EncodedFrameBuffer)
        .scaler(ScalerBuilder::new()
            .input_width(width as i32)
            .input_height(height as i32)
            .input_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_RGBA)
            .output_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P)
            .build()
        )
        .options(Options::new().set("crf", "26").set("tune", "zerolatency"))
        .build();

    let (decoder_pusher, decoder_puller) = DecoderBuilder::new()
        .codec_id("h264")
        .encoded_buffer_key(EncodedFrameBuffer)
        .decoded_buffer_key(DecodedRGBAFrameBuffer)
        .scaler(ScalerBuilder::new()
            .input_width(width as i32)
            .input_height(height as i32)
            .input_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P)
            .output_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_BGRA)
            .build()
        )
        .drain_error(NoFrame)
        .codec_error(CodecError)
        .build();

    let mut error_pipeline = Pipeline::<FrameData>::singleton(
        Component::new()
            .append(Function::new(|fd| {
                log::warn!("Dropped frame");
                Some(fd)
            }))
            .append(registry.get(CapturedRGBAFrameBuffer).redeemer().soft())
            .append(registry.get(EncodedFrameBuffer).redeemer().soft())
            .append(registry.get(DecodedRGBAFrameBuffer).redeemer().soft()),
    )
    .feedable();

    let pipeline = Pipeline::<FrameData>::new().link(
        Component::new()
            .append(Ticker::new(1000 / args.framerate))
            .append(registry.get(CapturedRGBAFrameBuffer).borrower())
            .append(capturer)
            .append(encoder_pusher)
            .append(registry.get(CapturedRGBAFrameBuffer).redeemer())
            .append(registry.get(EncodedFrameBuffer).borrower())
            .append(encoder_puller)
            .append(decoder_pusher)
            .append(registry.get(EncodedFrameBuffer).redeemer())
            .append(OnErrorSwitch::new(&mut error_pipeline))
            .append(registry.get(DecodedRGBAFrameBuffer).borrower())
            .append(decoder_puller)
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
