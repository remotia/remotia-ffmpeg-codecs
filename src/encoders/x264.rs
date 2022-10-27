#![allow(dead_code)]

use std::{ffi::CString, ptr::NonNull, sync::Arc};

use remotia::{traits::FrameProcessor, types::FrameData};
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::AVDictionary,
    ffi,
};

use async_trait::async_trait;

use cstr::cstr;
use tokio::sync::Mutex;

use super::{
    utils::frame_builders::yuv420p::YUV420PAVFrameBuilder,
    utils::{pull::pull_packet, push::push_frame}, options::Options,
};

pub struct X264Encoder {
    encode_context: Arc<Mutex<AVCodecContext>>,

    width: i32,
    height: i32,

    options: Options,
}

// TODO: Evaluate a safer way to move the encoder to another thread
// Necessary for multi-threaded pipelines
unsafe impl Send for X264Encoder {}

impl X264Encoder {
    pub fn new(width: i32, height: i32, options: Options) -> Self {
        let encoder = init_encoder(width, height, options.clone());
        let encode_context = Arc::new(Mutex::new(encoder));

        X264Encoder {
            width,
            height,

            options,
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
        let mut encode_context = self.encode_context.lock().await;
        push_frame(
            &mut encode_context,
            &mut self.yuv420_avframe_builder,
            &mut frame_data,
        );
        Some(frame_data)
    }
}

pub struct X264EncoderPuller {
    encode_context: Arc<Mutex<AVCodecContext>>,
}

#[async_trait]
impl FrameProcessor for X264EncoderPuller {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let mut encode_context = self.encode_context.lock().await;
        pull_packet(&mut encode_context, &mut frame_data);
        Some(frame_data)
    }
}

fn init_encoder(width: i32, height: i32, options: Options) -> AVCodecContext {
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

    let options_dict = options.to_av_dict();

    encode_context.open(Some(options_dict)).unwrap();
    encode_context
}
