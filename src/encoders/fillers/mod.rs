use rsmpeg::avutil::AVFrame;

pub mod rgba;
pub mod yuv420p;

pub trait AVFrameFiller<F> {
    fn fill(&mut self, frame_data: &F, avframe: &mut AVFrame);
}