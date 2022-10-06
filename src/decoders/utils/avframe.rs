pub fn write_avframe_to_yuv(
    avframe: rsmpeg::avutil::AVFrame,
    y_channel_buffer: &mut [u8],
    cb_channel_buffer: &mut [u8],
    cr_channel_buffer: &mut [u8],
) {
    let data = avframe.data;

    let height = avframe.height as usize;

    let linesize = avframe.linesize;
    let linesize_y = linesize[0] as usize;
    let linesize_cb = linesize[1] as usize;
    let linesize_cr = linesize[2] as usize;

    let y_data = unsafe { std::slice::from_raw_parts_mut(data[0], height * linesize_y) };
    let cb_data = unsafe { std::slice::from_raw_parts_mut(data[1], height / 2 * linesize_cb) };
    let cr_data = unsafe { std::slice::from_raw_parts_mut(data[2], height / 2 * linesize_cr) };

    y_channel_buffer.copy_from_slice(&y_data);
    cb_channel_buffer.copy_from_slice(&cb_data);
    cr_channel_buffer.copy_from_slice(&cr_data);
}
