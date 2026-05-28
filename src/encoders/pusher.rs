use std::sync::Arc;

use remotia::traits::FrameProcessor;
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::{scaling::Scaler, FFMpegCodec};

use super::fillers::AVFrameFiller;

pub struct EncoderPusher<T> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) filler: T,
    pub(super) eof_processed: bool,
}

#[async_trait]
impl<F, T> FrameProcessor<F> for EncoderPusher<T>
where
    T: AVFrameFiller<F> + Send,
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        if self.eof_processed {
            return None;
        }

        let mut encode_context = self.encode_context.lock().await;

        if frame_data.is_eof() {
            log::debug!("EncoderPusher: EOF, flushing encoder");
            encode_context.send_frame(None).ok();
            drain_packets(&mut encode_context, &mut frame_data);
            self.eof_processed = true;
            return Some(frame_data);
        }

        let input_avframe = self.scaler.input_frame_mut();
        if !self.filler.fill(&frame_data, input_avframe) {
            return None;
        }

        self.scaler.scale();
        self.scaler
            .scaled_frame_mut()
            .set_pts(frame_data.get_frame_id());

        let mut sent = false;
        while !sent {
            match encode_context.send_frame(Some(self.scaler.scaled_frame())) {
                Ok(()) => {
                    sent = true;
                }
                Err(RsmpegError::SendFrameAgainError) => {
                    drain_packets(&mut encode_context, &mut frame_data);
                }
                Err(e) => {
                    log::warn!("EncoderPusher: send_frame error: {:?}", e);
                    frame_data.report_codec_error();
                    sent = true;
                }
            }
        }

        Some(frame_data)
    }
}

fn drain_packets<F: FFMpegCodec>(encode_context: &mut AVCodecContext, frame_data: &mut F) {
    loop {
        match encode_context.receive_packet() {
            Ok(packet) => {
                let data =
                    unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
                frame_data.write_packet_data(data);
                frame_data.set_frame_id(packet.pts);
            }
            Err(RsmpegError::EncoderDrainError) => break,
            Err(RsmpegError::EncoderFlushedError) => {
                frame_data.report_flush_error();
                break;
            }
            Err(e) => {
                log::warn!("EncoderPusher: drain receive_packet error: {:?}", e);
                break;
            }
        }
    }
}
