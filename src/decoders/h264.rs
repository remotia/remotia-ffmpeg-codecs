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

use super::utils::packet::parse_packets;

use crate::ffi;

pub struct H264Decoder<K, E> {
    decode_context: AVCodecContext,
    scaling_context: SwsContext,
    parser_context: AVCodecParserContext,

    encoded_buffer_key: K,
    rgba_buffer_key: K,

    drain_error: E,
    codec_error: E,
}

// TODO: Fix all those unsafe impl
unsafe impl<K, E> Send for H264Decoder<K, E> {}

impl<K, E> H264Decoder<K, E> {
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

        H264Decoder {
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
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for H264Decoder<K, E>
where
    K: Send + Copy,
    E: Send + Copy,
    F: BorrowMutFrameProperties<K, BufferMut> + FrameError<E> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let timestamp = 0 as i64; // TODO: Extract timestamp from properties

        let encoded_buffer = frame_data.get_mut_ref(&self.encoded_buffer_key).unwrap();

        let encoded_packets_buffer = &encoded_buffer[..encoded_buffer.len()];

        let parse_result = parse_packets(
            &mut self.decode_context,
            &mut self.parser_context,
            encoded_packets_buffer,
            timestamp,
        );

        if let Err(error) = parse_result {
            debug!("Dropping frame, reason: {:?}", error);
            frame_data.report_error(self.codec_error);
            return Some(frame_data);
        }

        loop {
            match self.decode_context.receive_frame() {
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
