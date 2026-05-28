# remotia-ffmpeg-codecs

A generic API to add [FFmpeg](https://ffmpeg.org) codecs to [remotia](https://github.com/remotia/remotia) pipelines, built on top of the [rsmpeg](https://github.com/larksuite/rsmpeg) crate.

## Overview

This crate provides a **pusher/puller** architecture that integrates FFmpeg encoders and decoders into remotia's `FrameProcessor` pipeline model. Each codec (encoder or decoder) is split into two cooperating processors that run in separate pipeline components:

- **Encoders**: `EncoderPusher` sends raw frames into the FFmpeg encoder; `EncoderPuller` receives encoded packets back.
- **Decoders**: `DecoderPusher` feeds encoded packets into the FFmpeg decoder; `DecoderPuller` receives decoded frames back.

The pusher and puller are created together by their respective builders and share internal state, so they **must be placed in separate pipeline components** linked together.

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies.remotia-ffmpeg-codecs]
path = "../remotia-ffmpeg-codecs"
```

### Encoding

```text
[Capturer → EncoderPusher] → [EncoderPuller → PacketWriter]
      component 1                  component 2
```

```rust
use remotia_ffmpeg_codecs::encoders::EncoderBuilder;
use remotia_ffmpeg_codecs::encoders::fillers::rgba::RGBAFrameFiller;
use remotia_ffmpeg_codecs::scaling::ScalerBuilder;
use remotia_ffmpeg_codecs::options::Options;
use remotia_ffmpeg_codecs::ffi;

let scaler = ScalerBuilder::new()
    .input_width(1920)
    .input_height(1080)
    .input_pixel_format(ffi::AV_PIX_FMT_RGBA)
    .output_pixel_format(ffi::AV_PIX_FMT_YUV420P)
    .build();

let (encoder_pusher, encoder_puller) = EncoderBuilder::new()
    .codec_id("libx264")
    .filler(RGBAFrameFiller::new(BufferKey::RgbaFrame))
    .scaler(scaler)
    .options(Options::new().set("crf", "26").set("tune", "zerolatency"))
    .build();
```

### Decoding

```text
[ChunkReader → DecoderPusher] → [DecoderPuller → FrameWriter]
      component 1                      component 2
```

```rust
use remotia_ffmpeg_codecs::decoders::DecoderBuilder;
use remotia_ffmpeg_codecs::scaling::ScalerBuilder;
use remotia_ffmpeg_codecs::ffi;

let scaler = ScalerBuilder::new()
    .input_width(1920)
    .input_height(1080)
    .input_pixel_format(ffi::AV_PIX_FMT_YUV420P)
    .output_pixel_format(ffi::AV_PIX_FMT_RGBA)
    .build();

let (decoder_pusher, decoder_puller) = DecoderBuilder::new()
    .codec_id("h264")
    .scaler(scaler)
    .build();
```

## Implementing `FFMpegCodec`

Frame data types flowing through the pipeline must implement the `FFMpegCodec` trait, which bridges generic frame data with FFmpeg's packet/frame buffers. See the [h264_screen_mirror example](./examples/h264_screen_mirror/) and the [y4m-codec example](../examples/y4m-codec/) for complete implementations.

## Modules

- `encoders` — Encoder pusher/puller processors, builder, and frame fillers.
- `decoders` — Decoder pusher/puller processors and builder.
- `scaling` — Pixel format conversion and scaling via FFmpeg's `swscale`.
- `options` — Codec options as key/value pairs, convertible to `AVDictionary`.
- `ffi` — Re-export of `rsmpeg::ffi` for FFmpeg pixel format constants, etc.

## Examples

- [`h264_screen_mirror`](./examples/h264_screen_mirror/) — Screen capture → H.264 encode → decode → display loop.
- [`y4m-codec`](../examples/y4m-codec/) — Y4M file encoding and decoding to/from PNG frames.
- [`screen-stream`](../examples/screen-stream/) — Full screen streaming pipeline with SRT transport.
