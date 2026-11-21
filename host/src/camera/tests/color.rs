use super::*;

async fn encoded_color(
    color: &str,
    matrix: &str,
    range: &str,
    primaries: &str,
    transfer: &str,
) -> Vec<u8> {
    let filters =
        format!("scale=in_color_matrix=bt601:out_color_matrix={matrix}:out_range={range}");
    let tag = if matrix == "bt601" {
        "smpte170m"
    } else {
        "bt709"
    };
    let pixels = if range == "pc" { "yuvj420p" } else { "yuv420p" };
    let encoded = tokio::process::Command::new(executable("ffmpeg").unwrap())
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("color={color}:s=160x120:r=30"),
            "-vf",
            &filters,
            "-frames:v",
            "8",
            "-pix_fmt",
            pixels,
            "-c:v",
            "libx264",
            "-qp",
            "0",
            "-threads",
            "1",
            "-tune",
            "zerolatency",
            "-colorspace",
            tag,
            "-color_range",
            range,
            "-color_primaries",
            primaries,
            "-color_trc",
            transfer,
            "-f",
            "h264",
            "pipe:1",
        ])
        .output()
        .await
        .unwrap();
    assert!(
        encoded.status.success(),
        "{}",
        String::from_utf8_lossy(&encoded.stderr)
    );
    encoded.stdout
}

async fn check_color(color: &str, yuv: [u8; 3], rgb: [u8; 3]) {
    for matrix in ["bt601", "bt709"] {
        for range in ["tv", "pc"] {
            let encoded = encoded_color(color, matrix, range, "unknown", "unknown").await;
            let bytes = transformed_test_frame(&encoded, &options()).await;
            let actual = [bytes[0], bytes[19200], bytes[24000]];
            for (actual, expected) in actual.into_iter().zip(yuv) {
                assert!(
                    actual.abs_diff(expected) <= 3,
                    "T1456 {color} {matrix}/{range}: YUV {actual} != {expected}"
                );
            }
            let preview = blent_config::camera::CameraPreview::from_yuv420(
                blent_config::camera::Lens::Front,
                160,
                120,
                &bytes,
                std::time::Instant::now(),
            )
            .unwrap();
            for (actual, expected) in preview.rgba()[..3].iter().zip(rgb) {
                assert!(
                    actual.abs_diff(expected) <= 5,
                    "T1456 {color} {matrix}/{range}: RGB {actual} != {expected}"
                );
            }
        }
    }
}

#[tokio::test]
async fn t1456_tagged_matrices_and_ranges_share_exported_color_contract() {
    for (color, yuv, rgb) in [
        ("red", [81, 90, 240], [255, 0, 0]),
        ("lime", [145, 54, 34], [0, 255, 0]),
        ("blue", [41, 240, 110], [0, 0, 255]),
        ("black", [16, 128, 128], [0, 0, 0]),
        ("white", [235, 128, 128], [255, 255, 255]),
        ("gray", [126, 128, 128], [128, 128, 128]),
    ] {
        check_color(color, yuv, rgb).await;
    }
}

#[tokio::test]
async fn t1461_fully_tagged_bt709_primaries_are_converted_to_export_primaries() {
    let encoded = encoded_color("0x4080c0", "bt709", "tv", "bt709", "bt709").await;
    let bytes = transformed_test_frame(&encoded, &options()).await;
    let actual = [bytes[0], bytes[19200], bytes[24000]];
    // Independent fully tagged FFmpeg colorspace conversion, recorded in T1461.
    let expected = [112u8, 166, 92];
    for (value, expected) in actual.into_iter().zip(expected) {
        assert!(
            value.abs_diff(expected) <= 1,
            "T1461 SDR primaries: {actual:?}"
        );
    }
}

#[tokio::test]
async fn t1461_sdr_transfer_and_mixed_primaries_are_normalized() {
    for (primaries, transfer, expected) in [
        ("bt709", "iec61966-2-1", [100u8, 169, 89]),
        ("smpte170m", "bt709", [114, 165, 95]),
        ("bt470bg", "gamma28", [78, 174, 84]),
    ] {
        let encoded = encoded_color("0x4080c0", "bt709", "tv", primaries, transfer).await;
        let bytes = transformed_test_frame(&encoded, &options()).await;
        let actual = [bytes[0], bytes[19200], bytes[24000]];
        for (value, expected) in actual.into_iter().zip(expected) {
            assert!(
                value.abs_diff(expected) <= 1,
                "T1461 {primaries}/{transfer}: {actual:?}"
            );
        }
    }
}
