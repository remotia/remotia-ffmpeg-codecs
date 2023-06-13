use std::sync::Arc;

use bytes::BufMut;
use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext},
    avutil::{AVDictionary, AVFrame},
    error::RsmpegError,
    swscale::SwsContext,
};

use cstr::cstr;

use remotia::{
    buffers::BufferMut,
    traits::{BorrowMutFrameProperties, FrameError, FrameProcessor},
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::utils::packet::parse_and_send_packets;

use crate::ffi;

pub struct H264DecoderBuilder<K, E> {
    parser_context: AVCodecParserContext,
    decode_context: AVCodecContext,
    scaling_context: SwsContext,

    encoded_buffer_key: K,
    rgba_buffer_key: K,

    drain_error: E,
    codec_error: E,
}

// TODO: Fix all those unsafe impl
unsafe impl<K, E> Send for H264DecoderBuilder<K, E> {}

impl<K, E> H264DecoderBuilder<K, E> {
    pub fn new(
        width: i32,
        height: i32,
        encoded_buffer_key: K,
        rgba_buffer_key: K,
        drain_error: E,
        codec_error: E,
        input_pixel_format: ffi::AVPixelFormat,
        output_pixel_format: ffi::AVPixelFormat
    ) -> Self {
        let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).unwrap();

        let scaling_context = {
            SwsContext::get_context(
                width,
                height,
                input_pixel_format,
                width,
                height,
                output_pixel_format,
                ffi::SWS_BILINEAR,
            )
            .unwrap()
        };

        let options = AVDictionary::new(cstr!(""), cstr!(""), 0)
            .set(cstr!("threads"), cstr!("4"), 0)
            .set(cstr!("thread_type"), cstr!("slice"), 0);

        Self {
            decode_context: {
                let mut decode_context = AVCodecContext::new(&decoder);
                decode_context.open(Some(options)).unwrap();

                decode_context
            },

            scaling_context,

            parser_context: AVCodecParserContext::find(decoder.id).unwrap(),
            encoded_buffer_key,
            rgba_buffer_key,

            drain_error,
            codec_error
        }
    }

    pub fn build(self) -> (H264DecoderPusher<K, E>, H264DecoderPuller<K, E>) {
        let decode_context = Arc::new(Mutex::new(self.decode_context));

        (
            H264DecoderPusher {
                parser_context: self.parser_context,
                decode_context: decode_context.clone(),
                encoded_buffer_key: self.encoded_buffer_key,
                codec_error: self.codec_error
            },
            H264DecoderPuller {
                decode_context: decode_context.clone(),
                scaling_context: self.scaling_context,
                rgba_buffer_key: self.rgba_buffer_key,
                drain_error: self.drain_error,
            },
        )
    }
}

pub struct H264DecoderPusher<K, E> {
    parser_context: AVCodecParserContext,
    decode_context: Arc<Mutex<AVCodecContext>>,

    encoded_buffer_key: K,

    codec_error: E,
}

pub struct H264DecoderPuller<K, E> {
    decode_context: Arc<Mutex<AVCodecContext>>,
    scaling_context: SwsContext,
    rgba_buffer_key: K,
    drain_error: E,
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for H264DecoderPusher<K, E>
where
    K: Send + Copy,
    E: Send + Copy,
    F: BorrowMutFrameProperties<K, BufferMut> + FrameError<E> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let timestamp = 0 as i64; // TODO: Extract timestamp from properties

        let encoded_buffer = frame_data.get_mut_ref(&self.encoded_buffer_key).unwrap();

        let encoded_packets_buffer = &encoded_buffer[..encoded_buffer.len()];

        let mut decode_context = self.decode_context.lock().await;

        let send_result = parse_and_send_packets(
            &mut decode_context,
            &mut self.parser_context,
            encoded_packets_buffer,
            timestamp,
        );

        if let Err(error) = send_result {
            debug!("Dropping frame, reason: {:?}", error);
            frame_data.report_error(self.codec_error);
            return Some(frame_data);
        }

        Some(frame_data)
    }
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for H264DecoderPuller<K, E>
where
    K: Send + Copy,
    E: Send + Copy,
    F: BorrowMutFrameProperties<K, BufferMut> + FrameError<E> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        loop {
            let mut decode_context = self.decode_context.lock().await;
            match decode_context.receive_frame() {
                Ok(yuv_frame) => {
                    let mut rgba_frame = AVFrame::new();
                    rgba_frame.set_format(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGBA);
                    rgba_frame.set_width(yuv_frame.width);
                    rgba_frame.set_height(yuv_frame.height);
                    rgba_frame.set_pts(yuv_frame.pts);
                    rgba_frame.alloc_buffer().unwrap();

                    self.scaling_context
                        .scale_frame(&yuv_frame, 0, yuv_frame.height, &mut rgba_frame)
                        .unwrap();

                    let linesize = rgba_frame.linesize;
                    let height = rgba_frame.height as usize;

                    let linesize = linesize[0] as usize;
                    let data = unsafe { std::slice::from_raw_parts(rgba_frame.data[0], height * linesize) };

                    let rgba_buffer = frame_data.get_mut_ref(&self.rgba_buffer_key).unwrap();
                    rgba_buffer.put(data);

                    break;
                }
                Err(RsmpegError::DecoderDrainError) => {
                    debug!("No frames to be pulled");
                    frame_data.report_error(self.drain_error);
                    break;
                }
                Err(RsmpegError::DecoderFlushedError) => {
                    panic!("Decoder has been flushed unexpectedly");
                }
                Err(e) => panic!("{:?}", e),
            }
        }

        Some(frame_data)
    }
}
