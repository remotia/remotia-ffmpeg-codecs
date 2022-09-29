#![allow(dead_code)]

use std::{ffi::CString, ptr::NonNull, sync::Arc};

use log::debug;
use remotia::{traits::FrameProcessor, types::FrameData};
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::AVDictionary,
    ffi,
};

use async_trait::async_trait;

use cstr::cstr;
use tokio::sync::Mutex;

use super::{frame_builders::yuv420p::YUV420PAVFrameBuilder, receive_encoded_packet, send_avframe};

pub struct X264Encoder {
    encode_context: Arc<Mutex<AVCodecContext>>,

    width: i32,
    height: i32,

    x264opts: CString,
}

// TODO: Evaluate a safer way to move the encoder to another thread
// Necessary for multi-threaded pipelines
unsafe impl Send for X264Encoder {}

impl X264Encoder {
    pub fn new(width: i32, height: i32, x264opts: &str) -> Self {
        let x264opts = CString::new(x264opts.to_string()).unwrap();
        let encoder = init_encoder(width, height, 21, &x264opts);
        let encode_context = Arc::new(Mutex::new(encoder));

        X264Encoder {
            width,
            height,

            x264opts,
            encode_context,
        }
    }

    pub fn pusher(&self) -> X264EncoderPusher {
        X264EncoderPusher {
            encode_context: self.encode_context.clone(),
            yuv420_avframe_builder: YUV420PAVFrameBuilder::new(),
        }
    }

    pub fn puller(&self) -> X264EncoderPuller {
        X264EncoderPuller {
            encode_context: self.encode_context.clone(),
        }
    }
}

pub struct X264EncoderPusher {
    encode_context: Arc<Mutex<AVCodecContext>>,
    yuv420_avframe_builder: YUV420PAVFrameBuilder,
}

#[async_trait]
impl FrameProcessor for X264EncoderPusher {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let y_channel_buffer = frame_data
            .extract_writable_buffer("y_channel_buffer")
            .unwrap();

        let cb_channel_buffer = frame_data
            .extract_writable_buffer("cb_channel_buffer")
            .unwrap();

        let cr_channel_buffer = frame_data
            .extract_writable_buffer("cr_channel_buffer")
            .unwrap();

        let mut encode_context = self.encode_context.lock().await;

        let avframe = self.yuv420_avframe_builder.create_avframe(
            &mut encode_context,
            &y_channel_buffer,
            &cb_channel_buffer,
            &cr_channel_buffer,
            false,
        );

        send_avframe(&mut encode_context, avframe);

        frame_data.insert_writable_buffer("y_channel_buffer", y_channel_buffer);
        frame_data.insert_writable_buffer("cb_channel_buffer", cb_channel_buffer);
        frame_data.insert_writable_buffer("cr_channel_buffer", cr_channel_buffer);

        Some(frame_data)
    }
}

pub struct X264EncoderPuller {
    encode_context: Arc<Mutex<AVCodecContext>>,
}

#[async_trait]
impl FrameProcessor for X264EncoderPuller {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let mut output_buffer = frame_data
            .extract_writable_buffer("encoded_frame_buffer")
            .expect("No encoded frame buffer in frame DTO");

        let mut encode_context = self.encode_context.lock().await;

        let encoded_bytes = receive_encoded_packet(&mut encode_context, &mut output_buffer);

        frame_data.insert_writable_buffer("encoded_frame_buffer", output_buffer);

        frame_data.set("encoded_size", encoded_bytes as u128);

        Some(frame_data)
    }
}

fn init_encoder(width: i32, height: i32, crf: u32, x264opts: &CString) -> AVCodecContext {
    let encoder = AVCodec::find_encoder_by_name(cstr!("libx264")).unwrap();
    let mut encode_context = AVCodecContext::new(&encoder);
    encode_context.set_width(width);
    encode_context.set_height(height);
    encode_context.set_time_base(ffi::AVRational {
        num: 1,
        den: 60 * 1000,
    });
    encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
    encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
    let mut encode_context = unsafe {
        let raw_encode_context = encode_context.into_raw().as_ptr();
        AVCodecContext::from_raw(NonNull::new(raw_encode_context).unwrap())
    };

    let crf_str = format!("{}", crf);
    let crf_str = CString::new(crf_str).unwrap();

    let options = AVDictionary::new(cstr!(""), cstr!(""), 0)
        .set(cstr!("preset"), cstr!("ultrafast"), 0)
        .set(cstr!("crf"), &crf_str, 0)
        .set(cstr!("x264-params"), x264opts, 0)
        .set(cstr!("tune"), cstr!("zerolatency"), 0);

    encode_context.open(Some(options)).unwrap();
    encode_context
}
