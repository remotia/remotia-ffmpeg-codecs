use std::{
    ffi::{c_void, CString},
    sync::Arc,
};

use remotia::traits::{FrameProcessor, FrameProperties};
use rsmpeg::{
    avcodec::AVCodecContext,
    avutil::AVDictionary,
    ffi::{av_dict_set, av_dict_set_int, AVBuffer, AVBufferRef, AVRational},
    UnsafeDerefMut,
};

use async_trait::async_trait;

use tokio::sync::Mutex;

use crate::scaling::Scaler;

use super::fillers::AVFrameFiller;

pub struct EncoderPusher<T, P> {
    pub(super) encode_context: Arc<Mutex<AVCodecContext>>,
    pub(super) scaler: Scaler,
    pub(super) filler: T,
    pub(super) frame_id_prop: P,
}

#[async_trait]
impl<F, T, P> FrameProcessor<F> for EncoderPusher<T, P>
where
    T: AVFrameFiller<F> + Send,
    P: Send + Copy,
    F: FrameProperties<P, u128> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let frame_id = frame_data.get(&self.frame_id_prop).unwrap();

        let mut encode_context = self.encode_context.lock().await;

        let input_avframe = self.scaler.input_frame_mut();

        log::trace!("Input frame: {:#?}", input_avframe);

        self.filler.fill(&frame_data, input_avframe);

        self.scaler.scale();

        // unsafe {
        // let raw_ref = self.scaler.scaled_frame_mut().deref_mut();
        // let raw_ref = encode_context.as_mut_ptr();
        // let dict = AVDictionary::new(
        //     &CString::new("frame_id").unwrap(),
        //     &CString::new("test").unwrap(),
        //     0
        // ).into_raw();
        // }

        self.scaler.scaled_frame_mut().set_pts(frame_id as i64);

        /*unsafe {
            let raw = self.scaler.scaled_frame().as_ptr();
            // let raw = encode_context.as_ptr();
            let scaled_frame_data = (*raw).opaque_ref;
            log::debug!("Scaled frame data: {:?}", scaled_frame_data);
        }*/

        encode_context
            .send_frame(Some(&self.scaler.scaled_frame()))
            .unwrap();

        Some(frame_data)
    }
}
