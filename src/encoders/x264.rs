#![allow(dead_code)]

use std::{ptr::NonNull, sync::Arc};

use bytes::BytesMut;
use remotia::traits::{BorrowableFrameProperties, FrameProcessor, BorrowFrameProperties, PullableFrameProperties};
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    ffi,
};

use async_trait::async_trait;

use cstr::cstr;
use tokio::sync::Mutex;

use super::{
    options::Options,
    utils::frame_builders::yuv420p::YUV420PAVFrameBuilder,
    utils::{pull::pull_packet, push::push_frame},
};

pub struct X264Encoder<K: Copy> {
    encode_context: Arc<Mutex<AVCodecContext>>,

    width: i32,
    height: i32,

    y_buffer_key: K,
    cb_buffer_key: K,
    cr_buffer_key: K,
    encoded_buffer_key: K,

    options: Options,
}

// TODO: Evaluate a safer way to move the encoder to another thread
// Necessary for multi-threaded pipelines
unsafe impl<K: Copy> Send for X264Encoder<K> {}

impl<K: Copy> X264Encoder<K> {
    pub fn new(
        width: i32,
        height: i32,
        y_buffer_key: K,
        cb_buffer_key: K,
        cr_buffer_key: K,
        encoded_buffer_key: K,
        options: Options,
    ) -> Self {
        let encoder = init_encoder(width, height, options.clone());
        let encode_context = Arc::new(Mutex::new(encoder));

        X264Encoder {
            width,
            height,

            y_buffer_key,
            cb_buffer_key,
            cr_buffer_key,
            encoded_buffer_key,

            options,
            encode_context,
        }
    }

    pub fn pusher(&self) -> X264EncoderPusher<K> {
        X264EncoderPusher {
            encode_context: self.encode_context.clone(),
            yuv420_avframe_builder: YUV420PAVFrameBuilder::new(),
            y_buffer_key: self.y_buffer_key,
            cb_buffer_key: self.cb_buffer_key,
            cr_buffer_key: self.cr_buffer_key,
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
    yuv420_avframe_builder: YUV420PAVFrameBuilder,
    y_buffer_key: K,
    cb_buffer_key: K,
    cr_buffer_key: K,
}

/*
#[async_trait]
impl<'a, F, K> FrameProcessor<F> for X264EncoderPusher<K>
where
    K: Send + Copy,
    F: BorrowFrameProperties<K, &'a [u8]> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let mut encode_context = self.encode_context.lock().await;

        push_frame(
            &mut encode_context,
            &mut self.yuv420_avframe_builder,
            0 as i64,
            frame_data.get_ref(&self.y_buffer_key).unwrap(),
            frame_data.get_ref(&self.cb_buffer_key).unwrap(),
            frame_data.get_ref(&self.cr_buffer_key).unwrap()
        );

        Some(frame_data)
    }
}
*/

pub struct X264EncoderPuller<K> {
    encode_context: Arc<Mutex<AVCodecContext>>,
    encoded_buffer_key: K,
}

/*
#[async_trait]
impl<'a, F, K> FrameProcessor<F> for X264EncoderPuller<K> where
    K: Send,
    F: BorrowableFrameProperties<K, &'a mut [u8]> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut encode_context = self.encode_context.lock().await;
        pull_packet(
            &mut encode_context, 
            frame_data.get_mut_ref(&self.encoded_buffer_key).unwrap()
        );
        Some(frame_data)
    }
}
*/

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

