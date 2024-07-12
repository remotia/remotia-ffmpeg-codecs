use std::sync::Arc;

use remotia::{
    buffers::{BufMut, BytesMut},
    traits::{BorrowMutFrameProperties, FrameError, FrameProcessor, FrameProperties},
};
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use async_trait::async_trait;

use tokio::sync::Mutex;

pub struct EncoderPuller<K, EFE, P> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) encoded_buffer_key: K,
    pub(super) encoder_flushed_error: EFE,
    pub(super) frame_id_prop: P,
}

impl<K, EFE, P> EncoderPuller<K, EFE, P> {
    pub fn flusher_on<E>(&self, flush_error: E) -> EncoderFlusher<E> {
        EncoderFlusher {
            encode_context: self.encode_context.clone(),
            flush_error,
        }
    }
}

#[async_trait]
impl<'a, F, K, EFE, P> FrameProcessor<F> for EncoderPuller<K, EFE, P>
where
    K: Send,
    P: Send + Copy,
    EFE: Send + Copy,
    F: FrameError<EFE> + BorrowMutFrameProperties<K, BytesMut> + FrameProperties<P, u128> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        loop {
            let mut encode_context = self.encode_context.lock().await;
            let output_buffer = frame_data.get_mut_ref(&self.encoded_buffer_key).unwrap();

            let packet = match encode_context.receive_packet() {
                Ok(packet) => {
                    // debug!("Received packet of size {}", packet.size);
                    packet
                }
                Err(RsmpegError::EncoderDrainError) => {
                    log::debug!("Drain error, breaking the loop");
                    break;
                }
                Err(RsmpegError::EncoderFlushedError) => {
                    log::debug!("Flushed error, breaking the loop");
                    frame_data.report_error(self.encoder_flushed_error);
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };

            let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };

            unsafe {
                let raw = packet.as_ptr();
                log::debug!("Encoded packet: {:#?}", *raw);
            }

            output_buffer.put(data);
            
            let frame_id = packet.pts as u128;
            frame_data.set(self.frame_id_prop, frame_id);
        }
        Some(frame_data)
    }
}

pub struct EncoderFlusher<E> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(crate) flush_error: E,
}

#[async_trait]
impl<F, E> FrameProcessor<F> for EncoderFlusher<E>
where
    E: Send + Copy + std::cmp::PartialEq,
    F: FrameError<E> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        if let Some(error) = frame_data.get_error() {
            if error == self.flush_error {
                log::debug!("Received flush error, flushing encode context...");
                self.encode_context.lock().await.send_frame(None).unwrap();
            }
        }

        Some(frame_data)
    }
}
