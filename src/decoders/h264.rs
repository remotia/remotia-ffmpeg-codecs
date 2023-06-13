use std::sync::Arc;

use bytes::BufMut;
use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext},
    avutil::{AVFrame},
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

use crate::{encoders::options::Options, ffi};

pub struct H264DecoderBuilder<K, E> {
    encoded_buffer_key: Option<K>,
    decoded_buffer_key: Option<K>,

    drain_error: Option<E>,
    codec_error: Option<E>,

    options: Option<Options>,

    width: Option<i32>,
    height: Option<i32>,
    input_pixel_format: Option<ffi::AVPixelFormat>,
    output_pixel_format: Option<ffi::AVPixelFormat>,
    scaling_flags: Option<u32>,
}

// TODO: Fix all those unsafe impl
unsafe impl<K, E> Send for H264DecoderBuilder<K, E> {}

impl<K, E> H264DecoderBuilder<K, E> {
    pub fn new() -> Self {
        Self {
            encoded_buffer_key: None,
            decoded_buffer_key: None,
            drain_error: None,
            codec_error: None,
            width: None,
            height: None,
            input_pixel_format: None,
            output_pixel_format: None,
            scaling_flags: None,
            options: None,
        }
    }

    pub fn build(self) -> (H264DecoderPusher<K, E>, H264DecoderPuller<K, E>) {
        let options = self.options.unwrap_or_default().to_av_dict();

        let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).unwrap();
        let decode_context = {
            let mut decode_context = AVCodecContext::new(&decoder);
            decode_context.open(Some(options)).unwrap();

            Arc::new(Mutex::new(decode_context))
        };

        let width = self.width.expect("Missing mandatory field 'width'");
        let height = self.height.expect("Missing mandatory field 'height'");
        let input_pixel_format = self
            .input_pixel_format
            .expect("Missing mandatory field 'input_pixel_format'");
        let output_pixel_format = self
            .output_pixel_format
            .expect("Missing mandatory field 'output_pixel_format'");

        let scaling_flags = self.scaling_flags.unwrap_or(ffi::SWS_BILINEAR);

        let scaling_context = {
            SwsContext::get_context(
                width,
                height,
                input_pixel_format,
                width,
                height,
                output_pixel_format,
                scaling_flags,
            )
            .unwrap()
        };

        let parser_context = AVCodecParserContext::find(decoder.id).unwrap();

        let encoded_buffer_key = self
            .encoded_buffer_key
            .expect("Missing mandantory field 'encoded_buffer_key'");
        let codec_error = self
            .codec_error
            .expect("Missing mandatory field 'codec_error'");
        let decoded_buffer_key = self
            .decoded_buffer_key
            .expect("Missing mandantory field 'decoded_buffer_key'");
        let drain_error = self
            .drain_error
            .expect("Missing mandatory field 'drain_error'");

        (
            H264DecoderPusher {
                decode_context: decode_context.clone(),
                parser_context,
                encoded_buffer_key,
                codec_error,
            },
            H264DecoderPuller {
                decode_context: decode_context.clone(),
                scaling_context,
                decoded_buffer_key,
                drain_error,
            },
        )
    }

    pub fn encoded_buffer_key(mut self, encoded_buffer_key: K) -> Self {
        self.encoded_buffer_key = Some(encoded_buffer_key);
        self
    }

    pub fn decoded_buffer_key(mut self, decoded_buffer_key: K) -> Self {
        self.decoded_buffer_key = Some(decoded_buffer_key);
        self
    }

    pub fn drain_error(mut self, drain_error: E) -> Self {
        self.drain_error = Some(drain_error);
        self
    }

    pub fn codec_error(mut self, codec_error: E) -> Self {
        self.codec_error = Some(codec_error);
        self
    }

    pub fn options(mut self, options: Options) -> Self {
        self.options = Some(options);
        self
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: i32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn input_pixel_format(mut self, input_pixel_format: ffi::AVPixelFormat) -> Self {
        self.input_pixel_format = Some(input_pixel_format);
        self
    }

    pub fn output_pixel_format(mut self, output_pixel_format: ffi::AVPixelFormat) -> Self {
        self.output_pixel_format = Some(output_pixel_format);
        self
    }

    pub fn scaling_flags(mut self, scaling_flags: u32) -> Self {
        self.scaling_flags = Some(scaling_flags);
        self
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
    decoded_buffer_key: K,
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

                    let decoded_buffer = frame_data.get_mut_ref(&self.decoded_buffer_key).unwrap();
                    decoded_buffer.put(data);

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
