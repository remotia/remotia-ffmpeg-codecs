#![allow(dead_code)]

use std::{ptr::NonNull, sync::Arc};

use bytes::BytesMut;
use remotia::traits::{BorrowFrameProperties, BorrowMutFrameProperties, FrameProcessor};
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::AVFrame,
    ffi,
    swscale::SwsContext,
};

use async_trait::async_trait;

use cstr::cstr;
use tokio::sync::Mutex;

use super::{
    options::Options,
    utils::{push::push_frame, avframe::send_avframe},
    utils::{frame_builders::yuv420p::YUV420PAVFrameBuilder, packet::receive_encoded_packet},
};

pub struct X264Encoder<K: Copy> {
    encode_context: Arc<Mutex<AVCodecContext>>,
    scaling_context: Arc<Mutex<SwsContext>>,

    width: i32,
    height: i32,

    rgba_buffer_key: K,
    encoded_buffer_key: K,

    options: Options,
}

// TODO: Evaluate a safer way to move the encoder to another thread
// Necessary for multi-threaded pipelines
unsafe impl<K: Copy> Send for X264Encoder<K> {}

impl<K: Copy> X264Encoder<K> {
    pub fn new(width: i32, height: i32, rgba_buffer_key: K, encoded_buffer_key: K, options: Options) -> Self {
        let encoder = init_encoder(width, height, options.clone());
        let scaling_context = Arc::new(Mutex::new(
            SwsContext::get_context(
                encoder.width,
                encoder.height,
                rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGBA,
                encoder.width,
                encoder.height,
                encoder.pix_fmt,
                rsmpeg::ffi::SWS_BILINEAR,
            )
            .unwrap(),
        ));

        let encode_context = Arc::new(Mutex::new(encoder));

        X264Encoder {
            encode_context,
            scaling_context,

            width,
            height,

            rgba_buffer_key,
            encoded_buffer_key,

            options,
        }
    }

    pub fn pusher(&self) -> X264EncoderPusher<K> {
        X264EncoderPusher {
            encode_context: self.encode_context.clone(),
            scaling_context: self.scaling_context.clone(),
            rgba_buffer_key: self.rgba_buffer_key,
        }
    }

    pub fn puller(&self) -> X264EncoderPuller<K> {
        X264EncoderPuller {
            encode_context: self.encode_context.clone(),
            encoded_buffer_key: self.encoded_buffer_key,
        }
    }
}

pub struct X264EncoderPusher<K> {
    encode_context: Arc<Mutex<AVCodecContext>>,
    scaling_context: Arc<Mutex<SwsContext>>,

    rgba_buffer_key: K,
}

#[async_trait]
impl<F, K> FrameProcessor<F> for X264EncoderPusher<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let pts = 0 as i64; // TODO: Implement timestamp

        let mut encode_context = self.encode_context.lock().await;
        let mut scaling_context = self.scaling_context.lock().await;

        let mut rgba_frame = AVFrame::new();
        rgba_frame.set_format(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGBA);
        rgba_frame.set_width(encode_context.width);
        rgba_frame.set_height(encode_context.height);
        rgba_frame.set_pts(pts);
        rgba_frame.alloc_buffer().unwrap();
        let linesize = rgba_frame.linesize;
        let height = encode_context.height as usize;

        log::debug!("Linesize: {:?}", linesize);
        log::debug!("Height: {}", height);

        let linesize = linesize[0] as usize;
        let data = unsafe { std::slice::from_raw_parts_mut(rgba_frame.data[0], height * linesize) };

        log::debug!("Data len: {}", data.len());

        data.copy_from_slice(frame_data.get_ref(&self.rgba_buffer_key).unwrap());

        let mut yuv_frame = AVFrame::new();
        yuv_frame.set_format(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
        yuv_frame.set_width(encode_context.width);
        yuv_frame.set_height(encode_context.height);
        yuv_frame.set_pts(pts);
        yuv_frame.alloc_buffer().unwrap();

        scaling_context.scale_frame(&rgba_frame, 0, rgba_frame.height, &mut yuv_frame).unwrap();

        send_avframe(&mut encode_context, yuv_frame);

        Some(frame_data)
    }
}

pub struct X264EncoderPuller<K> {
    encode_context: Arc<Mutex<AVCodecContext>>,
    encoded_buffer_key: K,
}

#[async_trait]
impl<'a, F, K> FrameProcessor<F> for X264EncoderPuller<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut encode_context = self.encode_context.lock().await;
        receive_encoded_packet(&mut encode_context, frame_data.get_mut_ref(&self.encoded_buffer_key).unwrap());
        Some(frame_data)
    }
}

fn init_encoder(width: i32, height: i32, options: Options) -> AVCodecContext {
    let encoder = AVCodec::find_encoder_by_name(cstr!("libx264")).unwrap();
    let mut encode_context = AVCodecContext::new(&encoder);
    encode_context.set_width(width);
    encode_context.set_height(height);
    encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 * 1000 });
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
