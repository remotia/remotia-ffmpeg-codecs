pub mod pixel {
    pub fn yuv_to_bgr(_y: u8, _u: u8, _v: u8) -> (u8, u8, u8) {
        let y: f64 = _y as f64;
        let u: f64 = ((_u as i16) - 128) as f64;
        let v: f64 = ((_v as i16) - 128) as f64;

        let b = (y + u * 1.77200) as u8;
        let g = (y + u * -0.34414 + v * -0.71414) as u8;
        let r = (y + v * 1.40200) as u8;

        (b, g, r)
    }
}

pub mod raster {
    use super::pixel;

    pub fn yuv_to_bgr(yuv_pixels: &[u8], bgr_pixels: &mut [u8]) {
        let pixels_count = bgr_pixels.len() / 3;

        for i in 0..pixels_count {
            let (y, u, v) = (
                yuv_pixels[i],
                yuv_pixels[pixels_count + i / 4],
                yuv_pixels[pixels_count + pixels_count / 4 + i / 4],
            );

            let (b, g, r) = pixel::yuv_to_bgr(y, u, v);

            bgr_pixels[i * 3] = b;
            bgr_pixels[i * 3 + 1] = g;
            bgr_pixels[i * 3 + 2] = r;
        }
    }

    pub fn yuv_to_bgra(yuv_pixels: &[u8], bgra_pixels: &mut [u8]) {
        let pixels_count = bgra_pixels.len() / 4;

        for i in 0..pixels_count {
            let (y, u, v) = (
                yuv_pixels[i],
                yuv_pixels[pixels_count + i / 4],
                yuv_pixels[pixels_count + pixels_count / 4 + i / 4],
            );

            let (b, g, r) = pixel::yuv_to_bgr(y, u, v);

            bgra_pixels[i * 4] = b;
            bgra_pixels[i * 4 + 1] = g;
            bgra_pixels[i * 4 + 2] = r;
            bgra_pixels[i * 4 + 3] = 255;
        }
    }

    pub fn yuv_to_bgra_separate(
        y_pixels: &[u8],
        u_pixels: &[u8],
        v_pixels: &[u8],
        bgra_pixels: &mut [u8],
    ) {
        let pixels_count = bgra_pixels.len() / 4;

        for i in 0..pixels_count {
            let (y, u, v) = (y_pixels[i], u_pixels[i / 4], v_pixels[i / 4]);

            let (b, g, r) = pixel::yuv_to_bgr(y, u, v);

            bgra_pixels[i * 4] = b;
            bgra_pixels[i * 4 + 1] = g;
            bgra_pixels[i * 4 + 2] = r;
            bgra_pixels[i * 4 + 3] = 255;
        }
    }

    pub fn yuv_to_bgra_strided_separate(
        y_pixels: &[u8],
        u_pixels: &[u8],
        v_pixels: &[u8],
        width: usize,
        height: usize,
        y_stride: usize,
        u_stride: usize,
        v_stride: usize,
        bgra_pixels: &mut [u8],
    ) {
        for column in 0..height {
            for row in 0..width {
                let chroma_row = row / 4;
                let y_i = column * y_stride + row;
                let u_i = column * u_stride + chroma_row;
                let v_i = column * v_stride + chroma_row;
                let pixel_i = column * width + row;

                let (y, u, v) = (y_pixels[y_i], u_pixels[u_i], v_pixels[v_i]);

                let (b, g, r) = pixel::yuv_to_bgr(y, u, v);

                bgra_pixels[pixel_i * 4] = b;
                bgra_pixels[pixel_i * 4 + 1] = g;
                bgra_pixels[pixel_i * 4 + 2] = r;
                bgra_pixels[pixel_i * 4 + 3] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use log::debug;

    use crate::decoders::utils::yuv2bgr::raster;

    #[test]
    fn yuv_to_bgr_simple_test() {
        let input: Vec<u8> = vec![41, 0, 191, 79, 96, 116];
        let mut output: Vec<u8> = vec![0; input.len() * 2];

        raster::yuv_to_bgr(&input, &mut output);

        debug!("{:?}", output);
    }

    #[test]
    fn yuv_to_bgr_separate_test() {
        let y_data = vec![
            100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100,
        ];

        let cb_data = vec![15, 15, 15, 15];

        let cr_data = vec![10, 10, 10, 10];

        let mut output: Vec<u8> = vec![0; 16 * 4];

        raster::yuv_to_bgra_separate(&y_data, &cb_data, &cr_data, &mut output);

        let expected_output: Vec<u8> = vec![
            0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223,
            0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255,
            0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255,
        ];

        assert_eq!(output, expected_output);
    }

    #[test]
    fn yuv_to_bgr_strided_separate_test() {
        let y_data = vec![
            100, 100, 100, 100, 0, 0, 100, 100, 100, 100, 0, 0, 100, 100, 100, 100, 0, 0, 100, 100,
            100, 100, 0, 0,
        ];

        let cb_data = vec![15, 0, 0, 15, 0, 0, 15, 0, 0, 15, 0, 0];

        let cr_data = vec![10, 0, 0, 10, 0, 0, 10, 0, 0, 10, 0, 0];

        let mut output: Vec<u8> = vec![0; 16 * 4];

        raster::yuv_to_bgra_strided_separate(
            &y_data,
            &cb_data,
            &cr_data,
            4,
            4,
            6,
            3,
            3,
            &mut output,
        );

        let expected_output: Vec<u8> = vec![
            0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223,
            0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255,
            0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255, 0, 223, 0, 255,
        ];

        assert_eq!(output, expected_output);
    }
}
