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
        for row in 0..height {
            for column in 0..width {
                let chroma_row = row / 2;
                let chroma_column = column / 2;

                let y_i = row * y_stride + column;
                let u_i = chroma_row * u_stride + chroma_column;
                let v_i = chroma_row * v_stride + chroma_column;
                let pixel_i = row * width + column;

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

    fn yuv_data(y: u8, u: u8, v: u8, width: usize, height: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let y_data = vec![y; width * height];
        let u_data = vec![u; (width * height) / 4];
        let v_data = vec![v; (width * height) / 4];

        (y_data, u_data, v_data)
    }

    fn strided_row(value: u8, size: usize, stride: usize) -> Vec<u8> {
        let mut row = vec![0; stride];
        for i in 0..size {
            row[i] = value;
        }
        row
    }

    fn yuv_data_strided(
        y: u8,
        u: u8,
        v: u8,
        width: usize,
        height: usize,
        y_stride: usize,
        u_stride: usize,
        v_stride: usize,
    ) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let y_row = strided_row(y, width, y_stride);
        let u_row = strided_row(u, (width + 1) / 2, u_stride);
        let v_row = strided_row(v, (width + 1) / 2, v_stride);

        let mut y_data = vec![0; 0];
        for _ in 0..height {
            y_data.extend_from_slice(&y_row);
        }

        let mut u_data = vec![0; 0];
        for _ in 0..(height + 1) / 2 {
            u_data.extend_from_slice(&u_row);
        }

        let mut v_data = vec![0; 0];
        for _ in 0..(height + 1) / 2 {
            v_data.extend_from_slice(&v_row);
        }

        (y_data, u_data, v_data)
    }

    fn bgra_data(b: u8, g: u8, r: u8, a: u8, width: usize, height: usize) -> Vec<u8> {
        let mut rgba_data = vec![0; width * height * 4];
        for i in 0..width * height {
            rgba_data[i * 4] = b;
            rgba_data[i * 4 + 1] = g;
            rgba_data[i * 4 + 2] = r;
            rgba_data[i * 4 + 3] = a;
        }
        rgba_data
    }

    #[test]
    fn yuv_to_bgr_separate_test() {
        let (width, height) = (4, 4);
        let (y_data, cb_data, cr_data) = yuv_data(100, 15, 10, width, height);
        let mut output: Vec<u8> = vec![0; (width * height) * 4];
        raster::yuv_to_bgra_separate(&y_data, &cb_data, &cr_data, &mut output);
        let expected_output: Vec<u8> = bgra_data(0, 223, 0, 255, width, height);
        assert_eq!(output, expected_output);
    }

    fn yuv_to_bgr_fixed_strided_separate_test(
        width: usize,
        height: usize,
        y_stride: usize,
        u_stride: usize,
        v_stride: usize,
    ) {
        let (y_data, cb_data, cr_data) =
            yuv_data_strided(100, 15, 10, width, height, y_stride, u_stride, v_stride);
        let mut output: Vec<u8> = vec![0; (width * height) * 4];
        raster::yuv_to_bgra_strided_separate(
            &y_data,
            &cb_data,
            &cr_data,
            width,
            height,
            y_stride,
            u_stride,
            v_stride,
            &mut output,
        );
        let expected_output: Vec<u8> = bgra_data(0, 223, 0, 255, width, height);
        assert_eq!(output, expected_output);
    }

    #[test]
    fn yuv_to_bgr_4x3_strided_separate_test() {
        yuv_to_bgr_fixed_strided_separate_test(4, 3, 6, 3, 3);
    }

    #[test]
    fn yuv_to_bgr_4x4_strided_separate_test() {
        yuv_to_bgr_fixed_strided_separate_test(4, 4, 6, 3, 3);
    }

    #[test]
    fn yuv_to_bgr_12x7_strided_separate_test() {
        yuv_to_bgr_fixed_strided_separate_test(12, 7, 15, 16, 16);
    }

    #[test]
    fn yuv_to_bgr_128x72_strided_separate_test() {
        yuv_to_bgr_fixed_strided_separate_test(128, 72, 192, 96, 96);
    }

    #[test]
    fn yuv_to_bgr_1280x720_strided_1344_672_separate_test() {
        yuv_to_bgr_fixed_strided_separate_test(1280, 720, 1344, 672, 672);
    }
}
