use std::sync::Arc;

use rsmpeg::avcodec::{AVCodecContext, AVCodecParserContext};
use rsmpeg::error::RsmpegError;

use remotia::pipeline::PipelineHandle;
use remotia::traits::FrameProcessor;
use tokio::sync::mpsc::UnboundedSender;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{scaling::Scaler, FFMpegCodec};

pub struct DecoderPusher {
    pub(super) parser_context: AVCodecParserContext,
    pub(super) decode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) frame_tx: Option<UnboundedSender<Vec<u8>>>,
    pub(super) pipeline_handle: Option<PipelineHandle>,
    pub(super) eof_processed: bool,
}

#[async_trait]
impl<F> FrameProcessor<F> for DecoderPusher
where
    F: FFMpegCodec + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        if self.eof_processed {
            return None;
        }

        if frame_data.is_eof() {
            log::debug!("DecoderPusher: EOF, flushing parser and decoder");
            let mut decode_context = self.decode_context.lock().await;

            let mut packet = rsmpeg::avcodec::AVPacket::new();
            loop {
                let (packet_ready, consumed) = match self
                    .parser_context
                    .parse_packet(&mut decode_context, &mut packet, &[])
                {
                    Ok(result) => result,
                    Err(e) => {
                        log::warn!("DecoderPusher: parser flush error: {:?}", e);
                        break;
                    }
                };

                if consumed == 0 && !packet_ready {
                    break;
                }

                if packet_ready {
                    match decode_context.send_packet(Some(&packet)) {
                        Ok(()) => {}
                        Err(RsmpegError::DecoderFullError) => {
                            drain_frames(&mut decode_context, &mut self.scaler, &self.frame_tx);
                        }
                        Err(RsmpegError::DecoderFlushedError) => {}
                        Err(e) => {
                            log::warn!("DecoderPusher: flush send_packet error: {:?}", e);
                        }
                    }
                    packet = rsmpeg::avcodec::AVPacket::new();
                }
            }

            match decode_context.send_packet(None) {
                Ok(()) => {}
                Err(RsmpegError::DecoderFullError) => {
                    drain_frames(&mut decode_context, &mut self.scaler, &self.frame_tx);
                    if let Err(e) = decode_context.send_packet(None) {
                        log::warn!("DecoderPusher: retry send_packet(None) error: {:?}", e);
                    }
                }
                Err(e) => {
                    log::warn!("DecoderPusher: send_packet(None) error: {:?}", e);
                }
            }

            drain_frames(&mut decode_context, &mut self.scaler, &self.frame_tx);

            self.eof_processed = true;
            self.frame_tx.take();

            if let Some(handle) = &self.pipeline_handle {
                handle.request_shutdown();
            }

            return None;
        }

        let encoded_packets_buffer = frame_data.get_packet_data_buffer().to_vec();

        let mut decode_context = self.decode_context.lock().await;

        let mut packet = rsmpeg::avcodec::AVPacket::new();
        let mut parsed_offset = 0;

        while parsed_offset < encoded_packets_buffer.len() {
            let (packet_ready, consumed) = match self
                .parser_context
                .parse_packet(&mut decode_context, &mut packet, &encoded_packets_buffer[parsed_offset..])
            {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("DecoderPusher: parser error: {:?}", e);
                    break;
                }
            };

            parsed_offset += consumed;

            if packet_ready {
                let mut sent = false;
                while !sent {
                    match decode_context.send_packet(Some(&packet)) {
                        Ok(()) => {
                            sent = true;
                        }
                        Err(RsmpegError::DecoderFullError) => {
                            drain_frames(&mut decode_context, &mut self.scaler, &self.frame_tx);
                        }
                        Err(RsmpegError::DecoderFlushedError) => {
                            sent = true;
                        }
                        Err(e) => {
                            log::warn!("DecoderPusher: send_packet error: {:?}", e);
                            frame_data.report_codec_error();
                            sent = true;
                        }
                    }
                }

                packet = rsmpeg::avcodec::AVPacket::new();
            }
        }

        drain_frames(&mut decode_context, &mut self.scaler, &self.frame_tx);

        Some(frame_data)
    }
}

fn drain_frames(
    decode_context: &mut AVCodecContext,
    scaler: &mut Scaler,
    frame_tx: &Option<UnboundedSender<Vec<u8>>>,
) {
    let Some(frame_tx) = frame_tx else {
        return;
    };

    loop {
        match decode_context.receive_frame() {
            Ok(codec_avframe) => {
                scaler.scale_input(&codec_avframe);
                let output_avframe = scaler.scaled_frame_mut();
                let linesize = output_avframe.linesize[0] as usize;
                let height = output_avframe.height as usize;
                let data =
                    unsafe { std::slice::from_raw_parts(output_avframe.data[0], height * linesize) };

                if frame_tx.send(data.to_vec()).is_err() {
                    break;
                }
            }
            Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => break,
            Err(RsmpegError::ReceiveFrameError(-11)) => break,
            Err(e) => {
                log::warn!("DecoderPusher: drain receive_frame error: {:?}", e);
                break;
            }
        }
    }
}
