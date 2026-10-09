use super::*;

#[test]
fn t787_invalid_geometry_and_device_are_rejected_before_upload() {
    for width in [0, 1, 3, 4097, u32::MAX] {
        assert!(Frames::new("/nonexistent-t787", width, 64).is_err());
        assert!(Frames::new("/nonexistent-t787", 64, width).is_err());
    }
    assert!(Frames::new("device\0suffix", 64, 64).is_err());
    assert!(Frames::new("/nonexistent-t787", 64, 64).is_err());
    let error = super::super::Encoder::with_device(
        "h264_vaapi_baseline",
        64,
        64,
        60,
        500,
        18,
        1,
        "/nonexistent-t787",
    )
    .err()
    .expect("T787: selected missing device must fail");
    assert!(format!("{error:#}").contains("selected VAAPI device"));
}

#[test]
fn t787_plane_bounds_reject_missing_short_negative_and_outside_storage() {
    let mut frame = Video::new(Pixel::NV12, 64, 64);
    assert!(plane_valid(&frame, 0, 64, 64));
    assert!(plane_valid(&frame, 1, 64, 32));
    assert!(!plane_valid(&frame, usize::MAX, 64, 64));
    assert!(!plane_valid(&frame, 0, 64, 0));
    for rows in [65, 128, u32::MAX] {
        assert!(!plane_valid(&frame, 1, 64, rows));
    }
    unsafe {
        let raw = frame.as_mut_ptr();
        let stride = (*raw).linesize[0];
        for invalid in [i32::MIN, -1, 0, 63, i32::MAX] {
            (*raw).linesize[0] = invalid;
            assert!(!plane_valid(&frame, 0, 64, 64));
        }
        (*raw).linesize[0] = stride;
        let pixels = (*raw).data[0];
        (*raw).data[0] = ptr::null_mut();
        assert!(!plane_valid(&frame, 0, 64, 64));
        (*raw).data[0] = pixels;
        let buffer = (*raw).buf[0];
        (*raw).buf[0] = ptr::null_mut();
        assert!(!plane_valid(&frame, 0, 64, 64));
        (*raw).buf[0] = buffer;
    }
}

fn upload_case(node: &str) {
    let pool = Frames::new(&node, 64, 64).unwrap();
    let mut source = Video::new(Pixel::NV12, 64, 64);
    source.data_mut(0).fill(64);
    source.data_mut(1).fill(128);
    source.set_pts(Some(787));
    source.set_kind(ffmpeg_next::picture::Type::I);
    source.set_color_space(ffmpeg_next::color::Space::BT709);
    let uploaded = pool.upload(&source).unwrap();
    assert_eq!(uploaded.pts(), Some(787));
    assert_eq!(uploaded.kind(), ffmpeg_next::picture::Type::I);
    assert_eq!(uploaded.color_space(), ffmpeg_next::color::Space::BT709);
    source.data_mut(0).fill(192);
    let second = pool.upload(&source).unwrap();
    drop(pool);
    for (surface, expected) in [(uploaded, 64), (second, 192)] {
        let mut readback = Video::new(Pixel::NV12, 64, 64);
        unsafe {
            checked(
                ffi::av_hwframe_transfer_data(readback.as_mut_ptr(), surface.as_ptr(), 0),
                "read back test surface",
            )
            .unwrap();
        }
        for row in readback.data(0).chunks(readback.stride(0)).take(64) {
            assert!(row[..64].iter().all(|&b| b == expected));
        }
    }
}

fn rejection_case(node: &str) {
    let pool = Frames::new(&node, 64, 64).unwrap();
    assert!(pool.upload(&Video::new(Pixel::RGB24, 64, 64)).is_err());
    assert!(pool.upload(&Video::new(Pixel::NV12, 66, 64)).is_err());
    let mut empty = Video::empty();
    empty.set_format(Pixel::NV12);
    empty.set_width(64);
    empty.set_height(64);
    assert!(pool.upload(&empty).is_err());
    let mut broken = Video::new(Pixel::NV12, 64, 64);
    unsafe {
        (*broken.as_mut_ptr()).linesize[1] = 0;
    }
    assert!(pool.upload(&broken).is_err());
    let mut context = ffmpeg_next::codec::context::Context::new()
        .encoder()
        .video()
        .unwrap();
    pool.attach(&mut context).unwrap();
    assert!(pool.attach(&mut context).is_err());
    drop(pool);
    drop(context);
}

fn each_backend(case: impl Fn(&str)) {
    {
        let _guard = test_api::activate();
        case("portable-fake-device");
    }
    if let Ok(node) = std::env::var("BLENT_TEST_VAAPI_DEVICE") {
        case(&node);
    }
}

#[test]
fn t787_upload_retains_surface_and_metadata() {
    each_backend(upload_case);
}

#[test]
fn t787_input_rejection_and_pool_attachment() {
    each_backend(rejection_case);
}

#[test]
fn t787_native_allocation_and_device_failures_release_references() {
    let _guard = test_api::activate();
    for failure in 1..=3 {
        test_api::fail(failure);
        assert!(Frames::new("fake", 64, 64).is_err());
    }
    test_api::fail(0);
    let pool = Frames::new("fake", 64, 64).unwrap();
    let mut context = ffmpeg_next::codec::context::Context::new()
        .encoder()
        .video()
        .unwrap();
    test_api::fail(4);
    assert!(pool.attach(&mut context).is_err());
    let source = Video::new(Pixel::NV12, 64, 64);
    for failure in 5..=7 {
        test_api::fail(failure);
        assert!(pool.upload(&source).is_err());
        assert_eq!(unsafe { ffi::av_buffer_get_ref_count(pool.pool.0) }, 1);
    }
}

#[test]
fn t787_bounded_plane_layout_property() {
    let mut frame = Video::new(Pixel::NV12, 64, 64);
    unsafe {
        let raw = frame.as_mut_ptr();
        let pixels = (*raw).data[0];
        let stride = (*raw).linesize[0];
        let buffer = &*(*raw).buf[0];
        let base = buffer.data as usize;
        let size = buffer.size;
        for offset in [0, 1, 63, 64, size / 2, size - 1] {
            (*raw).data[0] = buffer.data.add(offset);
            for pitch in [-1, 0, 1, 63, 64, 65, 4096, i32::MAX] {
                (*raw).linesize[0] = pitch;
                for rows in [0, 1, 2, 32, 64, 65, u32::MAX] {
                    for width in [1, 2, 64, 65, u32::MAX] {
                        let extent = u64::from(rows.saturating_sub(1))
                            * u64::try_from(pitch).unwrap_or(0)
                            + u64::from(width);
                        let expected = rows > 0
                            && pitch > 0
                            && pitch as u32 >= width
                            && extent <= (size - offset) as u64;
                        assert_eq!(plane_valid(&frame, 0, width, rows), expected,
                            "T787: offset={offset} pitch={pitch} rows={rows} width={width} base={base}");
                    }
                }
            }
        }
        (*raw).data[0] = pixels;
        (*raw).linesize[0] = stride;
    }
}
