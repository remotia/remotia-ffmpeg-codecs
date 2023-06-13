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

        Scaler { sws_context }
    }
}

pub struct Scaler {
    sws_context: SwsContext,
}

impl Scaler {
    pub fn scale(&mut self, input_frame: &AVFrame, output_frame: &mut AVFrame) {
        self.sws_context
            .scale_frame(&input_frame, 0, input_frame.height, output_frame)
            .unwrap();
    }
}
