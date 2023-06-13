use std::sync::Arc;

use bytes::BytesMut;
use remotia::traits::{BorrowMutFrameProperties, FrameProcessor};
use rsmpeg::{
    avcodec::{AVCodecContext},
};

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::encoders::utils::packet::receive_encoded_packet;


pub struct X264EncoderPuller<K> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) encoded_buffer_key: K,
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
