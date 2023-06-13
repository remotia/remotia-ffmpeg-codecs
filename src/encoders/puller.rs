use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use remotia::traits::{BorrowMutFrameProperties, FrameProcessor};
use rsmpeg::{avcodec::AVCodecContext, error::RsmpegError};

use async_trait::async_trait;

use tokio::sync::Mutex;

pub struct EncoderPuller<K> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) encoded_buffer_key: K,
}

#[async_trait]
impl<'a, F, K> FrameProcessor<F> for EncoderPuller<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
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
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };

            let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };

            log::debug!("Encoded packet: {:?}", packet);

            output_buffer.put(data);
        }
        Some(frame_data)
    }
}
