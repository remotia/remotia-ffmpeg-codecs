use crate::{builder::unwrap_mandatory, ffi};
use rsmpeg::{avutil::AVFrame, swscale::SwsContext};

pub struct ScalerBuilder {
    input_width: Option<i32>,
    input_height: Option<i32>,
    input_pixel_format: Option<ffi::AVPixelFormat>,
    output_width: Option<i32>,
    output_height: Option<i32>,
    output_pixel_format: Option<ffi::AVPixelFormat>,
    scaling_flags: Option<u32>,
}

impl ScalerBuilder {
    pub fn new() -> Self {
        Self {
            input_width: None,
            input_height: None,
            input_pixel_format: None,
            output_width: None,
            output_height: None,
            output_pixel_format: None,
            scaling_flags: None,
        }
    }

    builder_set!(input_width, i32);
    builder_set!(input_height, i32);
    builder_set!(output_width, i32);
    builder_set!(output_height, i32);
    builder_set!(input_pixel_format, ffi::AVPixelFormat);
    builder_set!(output_pixel_format, ffi::AVPixelFormat);
    builder_set!(scaling_flags, u32);

    pub fn build(self) -> Scaler {
        let input_width = unwrap_mandatory(self.input_width);
        let input_height = unwrap_mandatory(self.input_height);
        let input_pixel_format = unwrap_mandatory(self.input_pixel_format);

        let output_width = self.output_width.unwrap_or(input_width);
        let output_height = self.output_height.unwrap_or(input_height);
        let output_pixel_format = unwrap_mandatory(self.output_pixel_format);

        let scaling_flags = self.scaling_flags.unwrap_or(ffi::SWS_BILINEAR);

        let sws_context = {
            SwsContext::get_context(
                input_width,
                input_height,
                input_pixel_format,
                output_width,
                output_height,
                output_pixel_format,
                scaling_flags,
            )
            .unwrap()
        };

        let input_avframe = {
            let mut avframe = AVFrame::new();
            avframe.set_format(input_pixel_format);
            avframe.set_width(input_width);
            avframe.set_height(input_height);
            avframe.alloc_buffer().unwrap();
            avframe
        };

        let output_avframe = {
            let mut avframe = AVFrame::new();
            avframe.set_format(output_pixel_format);
            avframe.set_width(output_width);
            avframe.set_height(output_height);
            avframe.alloc_buffer().unwrap();
            avframe
        };

        Scaler {
            input_frame: input_avframe,
            scaled_frame: output_avframe,
            sws_context,
        }
    }
}

pub struct Scaler {
    sws_context: SwsContext,
    input_frame: AVFrame,
    scaled_frame: AVFrame,
}
impl Scaler {
    pub fn scale(&mut self) {
        let input_frame = &self.input_frame;
        let scaled_frame = &mut self.scaled_frame;

        self.sws_context
            .scale_frame(input_frame, 0, input_frame.height, scaled_frame)
            .unwrap();

        scaled_frame.set_pts(input_frame.pts);
    }

    pub fn scale_input(&mut self, input_frame: &AVFrame) {
        let scaled_frame = &mut self.scaled_frame;

        self.sws_context
            .scale_frame(input_frame, 0, input_frame.height, scaled_frame)
            .unwrap();

        scaled_frame.set_pts(input_frame.pts);
    }

    pub fn input_frame(&self) -> &AVFrame {
        &self.input_frame
    }

    pub fn scaled_frame(&self) -> &AVFrame {
        &self.scaled_frame
    }

    pub fn input_frame_mut(&mut self) -> &mut AVFrame {
        &mut self.input_frame
    }

    pub fn scaled_frame_mut(&mut self) -> &mut AVFrame {
        &mut self.scaled_frame
    }
}
