//! Host filesystem adapter for the shared fixed-porch EDID encoder.
use anyhow::{Context, Result};
pub use blent_config::display::{DEFAULT_HEIGHT_MM, DEFAULT_WIDTH_MM};
use std::path::PathBuf;

#[cfg(test)]
fn make_edid_sized(
    width: u32,
    height: u32,
    refresh: u32,
    width_mm: u32,
    height_mm: u32,
    instance: u32,
) -> Result<Vec<u8>> {
    Ok(blent_config::edid::Mode {
        width,
        height,
        refresh,
        width_mm,
        height_mm,
        instance,
    }
    .encode("Blent")?
    .to_vec())
}

/// Build an EDID at the default physical size.
#[cfg(test)]
pub fn make_edid(width: u32, height: u32, refresh: u32) -> Vec<u8> {
    make_edid_sized(
        width,
        height,
        refresh,
        DEFAULT_WIDTH_MM,
        DEFAULT_HEIGHT_MM,
        0,
    )
    .unwrap()
}

/// Write (or reuse) a generated EDID for this mode and return its path.
pub fn ensure_edid_sized(
    width: u32,
    height: u32,
    refresh: u32,
    width_mm: u32,
    height_mm: u32,
    instance: u32,
) -> Result<PathBuf> {
    let dir = blent_config::platform::data_dir()?.join("edid");
    ensure_edid_in(&dir, width, height, refresh, width_mm, height_mm, instance)
}

fn ensure_edid_in(
    dir: &std::path::Path,
    width: u32,
    height: u32,
    refresh: u32,
    width_mm: u32,
    height_mm: u32,
    instance: u32,
) -> Result<PathBuf> {
    let edid = blent_config::edid::Mode {
        width,
        height,
        refresh,
        width_mm,
        height_mm,
        instance,
    }
    .encode("Blent")?;
    std::fs::create_dir_all(dir).context("create EDID dir")?;
    let path = dir.join(format!(
        "auto-slot{}-{}x{}@{}-{}x{}mm.bin",
        instance, width, height, refresh, width_mm, height_mm
    ));
    if std::fs::read(&path).ok().as_deref() != Some(edid.as_slice()) {
        // Only complete, matching generated contents are reusable. A previous
        // interrupted write must not poison every subsequent capture attempt.
        use std::io::Write;
        let mut temporary = tempfile::NamedTempFile::new_in(dir).context("create EDID file")?;
        temporary.write_all(&edid).context("write EDID")?;
        temporary.as_file().sync_all().context("sync EDID")?;
        temporary.persist(&path).context("replace EDID")?;
        tracing::info!(
            "Generated EDID for {}x{}@{} ({}x{}mm) at {:?}",
            width,
            height,
            refresh,
            width_mm,
            height_mm,
            path
        );
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn t585_blent_monitor_descriptor_has_fixed_width_and_checksum() {
        let edid = super::make_edid(1280, 800, 60);
        assert_eq!(&edid[95..108], b"Blent\n       ");
        assert_eq!(&edid[8..10], &[0x09, 0x8e]);
        assert_eq!(
            edid.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
            0
        );
    }

    use super::*;

    #[test]
    fn t321_range_minimum_tracks_supported_geometry() {
        let rust = make_edid_sized(640, 480, 25, 310, 194, 0).unwrap();
        let python = python_edid(640, 480, 25, 310, 194).unwrap();
        assert_eq!(
            rust[113], 25,
            "T321: Rust range advertises unsupported low refresh"
        );
        assert_eq!(
            python[113], 25,
            "T321: Python range advertises unsupported low refresh"
        );
        assert_eq!(rust, python);
    }

    #[test]
    fn t321_python_rejects_low_clock_modes() {
        for fps in [10, 24] {
            assert!(
                python_edid(640, 480, fps, 310, 194).is_none(),
                "T321: Python admitted low clock"
            );
        }
        assert!(python_edid(640, 480, 25, 310, 194).is_some());
    }

    #[test]
    fn t321_rust_rejects_low_clock_modes() {
        for fps in [10, 24] {
            assert!(
                make_edid_sized(640, 480, fps, 310, 194, 0).is_err(),
                "T321: Rust admitted low clock"
            );
        }
        assert!(make_edid_sized(640, 480, 25, 310, 194, 0).is_ok());
    }

    fn assert_t320_dummy_descriptor(edid: &[u8]) {
        // EDID base-block display descriptor: zero clock/reserved bytes,
        // dummy tag 0x10, and zero reserved payload. All-zero is invalid.
        assert_eq!(edid.len(), 128);
        assert_eq!(&edid[72..75], &[0, 0, 0]);
        assert_eq!(edid[75], 0x10, "T320: unused descriptor has no dummy tag");
        assert!(edid[76..90].iter().all(|&byte| byte == 0));
        assert_eq!(
            edid.iter().map(|&byte| u32::from(byte)).sum::<u32>() % 256,
            0
        );
    }

    #[test]
    fn t320_rust_unused_descriptor_is_valid() {
        for (width, height, fps) in [(1920, 1080, 60), (2960, 1848, 90), (1024, 4095, 10)] {
            assert_t320_dummy_descriptor(&make_edid(width, height, fps));
        }
    }

    #[test]
    fn t320_python_unused_descriptor_is_valid() {
        for (width, height, fps) in [(1920, 1080, 60), (2960, 1848, 90), (1024, 4095, 10)] {
            assert_t320_dummy_descriptor(&python_edid(width, height, fps, 310, 194).unwrap());
        }
    }

    #[test]
    fn t246_repairs_incomplete_and_wrong_generated_edids() {
        use std::os::unix::fs::MetadataExt;
        let dir = tempfile::tempdir().unwrap();
        let expected = make_edid_sized(1920, 1080, 60, 310, 194, 0).unwrap();
        let path = ensure_edid_in(dir.path(), 1920, 1080, 60, 310, 194, 0).unwrap();
        let wrong_mode = make_edid_sized(1280, 720, 60, 310, 194, 0).unwrap();
        for corrupt in [vec![], expected[..64].to_vec(), wrong_mode] {
            std::fs::write(&path, &corrupt).unwrap();
            assert_eq!(
                ensure_edid_in(dir.path(), 1920, 1080, 60, 310, 194, 0).unwrap(),
                path
            );
            assert_eq!(
                std::fs::read(&path).unwrap(),
                expected,
                "T246: damaged generated EDID was reused"
            );
        }
        let valid_inode = std::fs::metadata(&path).unwrap().ino();
        ensure_edid_in(dir.path(), 1920, 1080, 60, 310, 194, 0).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().ino(),
            valid_inode,
            "valid cache should be reused"
        );
    }

    #[test]
    fn t246_cache_directory_is_not_returned_as_an_edid() {
        let dir = tempfile::tempdir().unwrap();
        let path = ensure_edid_in(dir.path(), 1920, 1080, 60, 310, 194, 0).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(
            ensure_edid_in(dir.path(), 1920, 1080, 60, 310, 194, 0).is_err(),
            "T246: helper would be given a directory instead of an EDID"
        );
    }

    // Independent DTD decoder: Linux drm_edid.h orders width low, height
    // low, then both high nibbles at offsets 12, 13, 14.
    fn physical_size(edid: &[u8]) -> (u32, u32) {
        (
            u32::from(edid[66]) | (u32::from(edid[68] & 0xf0) << 4),
            u32::from(edid[67]) | (u32::from(edid[68] & 0x0f) << 8),
        )
    }

    #[test]
    fn t114_rust_dtd_reports_exact_physical_size() {
        for (w, h) in [(310, 194), (255, 256), (4095, 4095)] {
            assert_eq!(
                physical_size(&make_edid_sized(1920, 1080, 60, w, h, 0).unwrap()),
                (w, h)
            );
        }
    }

    #[test]
    fn t114_t1702_current_cache_repairs_wrong_physical_size() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join("auto-slot0-1920x1080@60-310x194mm.bin");
        let wrong = make_edid_sized(1920, 1080, 60, 255, 256, 0).unwrap();
        std::fs::write(&current, wrong).unwrap();
        let path = ensure_edid_in(dir.path(), 1920, 1080, 60, 310, 194, 0).unwrap();
        assert_eq!(path, current, "T1702 must repair the current mode identity");
        assert_eq!(physical_size(&std::fs::read(path).unwrap()), (310, 194));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn t114_python_dtd_reports_exact_physical_size() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("display.bin");
        let script =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/gen-edid.py");
        assert!(std::process::Command::new("python3")
            .arg(script)
            .args(["1920", "1080", "60"])
            .arg(&output)
            .output()
            .unwrap()
            .status
            .success());
        assert_eq!(physical_size(&std::fs::read(output).unwrap()), (310, 194));
    }

    fn python_edid(w: u32, h: u32, fps: u32, wm: u32, hm: u32) -> Option<Vec<u8>> {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("display.bin");
        let script =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/gen-edid.py");
        let status = std::process::Command::new("python3")
            .arg(script)
            .args([w.to_string(), h.to_string(), fps.to_string()])
            .arg(&output)
            .args([wm.to_string(), hm.to_string()])
            .output()
            .unwrap()
            .status;
        status.success().then(|| std::fs::read(output).unwrap())
    }

    #[test]
    fn t332_shared_mode_validation_agrees_with_both_generators_at_clock_boundary() {
        for (width, height, fps, valid) in [
            (3840, 2160, 60, true),
            (3840, 2160, 74, true),
            (3840, 2160, 75, false),
            (3840, 2160, 90, false),
            (4095, 4095, 60, false),
            (640, 480, 10, false),
            (640, 480, 25, true),
        ] {
            let config = blent_config::FileConfig {
                width,
                height,
                fps,
                ..Default::default()
            };
            let rust = make_edid_sized(width, height, fps, 310, 194, 0);
            let python = python_edid(width, height, fps, 310, 194);
            assert_eq!(
                config.validate().is_ok(),
                valid,
                "T332: {width}x{height}@{fps}"
            );
            assert_eq!(rust.is_ok(), valid);
            assert_eq!(python.is_some(), valid);
            if valid {
                assert_eq!(rust.unwrap(), python.unwrap());
            }
        }
    }

    #[test]
    fn t115_generators_agree_on_boundaries_color_and_ranges() {
        for (w, h, fps, wm, hm) in [
            (1920, 1080, 10, 310, 194),
            (2960, 1848, 90, 314, 195),
            (4095, 2160, 30, 4095, 4095),
            (1, 1080, 60, 255, 256),
            (1024, 4095, 90, 310, 194),
        ] {
            let rust = make_edid_sized(w, h, fps, wm, hm, 0).unwrap();
            let python = python_edid(w, h, fps, wm, hm).unwrap();
            assert_eq!(rust, python, "generator mismatch at {w}x{h}@{fps}");
            assert!(u32::from(rust[113]) <= fps && u32::from(rust[114]) >= fps);
            let pclk = u32::from(u16::from_le_bytes([rust[54], rust[55]])) * 10_000;
            let horizontal = pclk as f64 / f64::from(w + 160);
            let low = u32::from(rust[115]) + if rust[112] & 4 != 0 { 255 } else { 0 };
            let high = u32::from(rust[116]) + if rust[112] & 8 != 0 { 255 } else { 0 };
            assert!(f64::from(low * 1000) <= horizontal && f64::from(high * 1000) >= horizontal);
            // sRGB xy coordinates decoded independently from packed 10-bit values.
            let expected = [0.640, 0.330, 0.300, 0.600, 0.150, 0.060, 0.3127, 0.3290];
            for (index, expected) in expected.into_iter().enumerate() {
                let low_bits = (rust[25 + index / 4] >> (6 - (index % 4) * 2)) & 3;
                let value = (u16::from(rust[27 + index]) * 4 + u16::from(low_bits)) as f64 / 1024.0;
                assert!((value - expected).abs() < 1.0 / 1024.0);
            }
        }
    }

    #[test]
    fn t115_both_generators_reject_unrepresentable_or_unsupported_values() {
        for (w, h, fps, wm, hm) in [
            (1, 1080, 10, 255, 256),
            (4096, 1080, 60, 310, 194),
            (1920, 4096, 60, 310, 194),
            (0, 1080, 60, 310, 194),
            (1920, 1080, 0, 310, 194),
            (1920, 1080, 9, 310, 194),
            (1920, 1080, 91, 310, 194),
            (4095, 4095, 90, 310, 194),
            (1920, 1080, 60, 4096, 194),
            (1920, 1080, 60, 310, 0),
        ] {
            assert!(
                python_edid(w, h, fps, wm, hm).is_none(),
                "Python accepted {w}x{h}@{fps}, {wm}x{hm} mm"
            );
            assert!(
                make_edid_sized(w, h, fps, wm, hm, 0).is_err(),
                "Rust accepted {w}x{h}@{fps}, {wm}x{hm} mm"
            );
        }
    }

    #[test]
    fn t115_t1702_current_cache_repairs_wrong_refresh_range() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join("auto-slot0-1920x1080@10-310x194mm.bin");
        let mut wrong = make_edid_sized(1920, 1080, 10, 310, 194, 0).unwrap();
        wrong[113] = 0;
        wrong[114] = 240;
        wrong[127] = 0u8.wrapping_sub(wrong[..127].iter().fold(0u8, |sum, b| sum.wrapping_add(*b)));
        std::fs::write(&current, wrong).unwrap();
        let path = ensure_edid_in(dir.path(), 1920, 1080, 10, 310, 194, 0).unwrap();
        assert_eq!(path, current, "T1702 must repair the current mode identity");
        let actual = std::fs::read(path).unwrap();
        assert_eq!(&actual[113..115], &[10, 90]);
        assert_eq!(actual.iter().fold(0u8, |sum, b| sum.wrapping_add(*b)), 0);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn t061_rejects_unrepresentable_edid_modes() {
        for (w, h, fps) in [
            (4096, 1080, 60),
            (1920, 4096, 60),
            (4095, 4095, 90),
            (0, 1080, 60),
        ] {
            assert!(
                make_edid_sized(w, h, fps, 310, 194, 0).is_err(),
                "{w}x{h}@{fps} must not be truncated"
            );
        }
        assert!(make_edid_sized(4095, 2160, 30, 310, 194, 0).is_ok());
    }

    #[test]
    fn edid_checksum_is_valid() {
        for (w, h) in [(2960u32, 1848u32), (2560, 1600), (1920, 1200), (2000, 1200)] {
            let edid = make_edid(w, h, 60);
            assert_eq!(edid.len(), 128);
            let sum: u32 = edid.iter().map(|&b| b as u32).sum();
            assert_eq!(sum % 256, 0, "checksum for {}x{}", w, h);
            // Decode DTD active size back
            let h_act = edid[56] as u32 | (((edid[58] >> 4) as u32) << 8);
            let v_act = edid[59] as u32 | (((edid[61] >> 4) as u32) << 8);
            assert_eq!((h_act, v_act), (w, h));
        }
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    #[test]
    fn t1496_instance_serials_are_injective_and_reject_overflow() {
        let mut serials = std::collections::BTreeSet::new();
        for instance in (0..1024).chain([u32::MAX - 1]) {
            let edid = make_edid_sized(1280, 800, 60, 310, 194, instance).unwrap();
            let serial = u32::from_le_bytes(edid[12..16].try_into().unwrap());
            assert_eq!(serial, instance + 1);
            assert!(serials.insert(serial));
            assert_eq!(
                edid.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
                0
            );
        }
        assert!(make_edid_sized(1280, 800, 60, 310, 194, u32::MAX).is_err());
    }
}

#[cfg(test)]
mod location_tests;
