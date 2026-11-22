use super::*;

fn mode() -> Mode {
    Mode {
        width: 1920,
        height: 1080,
        refresh: 60,
        width_mm: 310,
        height_mm: 194,
        instance: 0,
    }
}

#[test]
fn t1717_names_are_bounded_terminated_and_checksummed() {
    for (name, expected) in [
        ("Blent", b"Blent\n       ".as_slice()),
        ("", b"\n            ".as_slice()),
        ("abcdefghijklmnop", b"abcdefghijkl\n".as_slice()),
        ("á\u{1f601}B", b"??B\n         ".as_slice()),
    ] {
        let block = mode().encode(name).unwrap();
        assert_eq!(&block[95..108], expected);
        assert_eq!(block.iter().fold(0u8, |sum, b| sum.wrapping_add(*b)), 0);
        assert_eq!(&block[..8], &[0, 255, 255, 255, 255, 255, 255, 0]);
        assert_eq!(&block[8..10], &[0x09, 0x8e]);
    }
}

#[test]
fn t1717_identity_and_physical_boundaries_never_wrap() {
    for value in [0, 1, 255, 256, 4095, 4096, u32::MAX] {
        let request = Mode {
            width_mm: value,
            height_mm: value,
            ..mode()
        };
        let block = request.encode("Blent");
        assert_eq!(block.is_ok(), (1..=4095).contains(&value));
        if let Ok(block) = block {
            assert_eq!(
                u32::from(block[66]) | (u32::from(block[68] >> 4) << 8),
                value
            );
            assert_eq!(
                u32::from(block[67]) | (u32::from(block[68] & 15) << 8),
                value
            );
        }
    }
    for instance in (0..1024).chain([u32::MAX - 1]) {
        let block = Mode { instance, ..mode() }.encode("Blent").unwrap();
        assert_eq!(
            u32::from_le_bytes(block[12..16].try_into().unwrap()),
            instance + 1
        );
    }
    assert!(Mode {
        instance: u32::MAX,
        ..mode()
    }
    .encode("Blent")
    .is_err());
}

#[test]
fn t1717_current_geometry_boundaries_preserve_independent_fields() {
    for (width, height, refresh, valid) in [
        (640, 480, 24, false),
        (640, 480, 25, true),
        (3840, 2160, 74, true),
        (3840, 2160, 75, false),
        (4096, 1080, 60, false),
        (0, 1080, 60, false),
        (1920, 1080, 91, false),
    ] {
        let result = Mode {
            width,
            height,
            refresh,
            ..mode()
        }
        .encode("Blent");
        assert_eq!(result.is_ok(), valid);
        if let Ok(block) = result {
            assert_eq!(
                u32::from(block[56]) | (u32::from(block[58] >> 4) << 8),
                width
            );
            assert_eq!(
                u32::from(block[59]) | (u32::from(block[61] >> 4) << 8),
                height
            );
            assert!(u32::from(block[113]) <= refresh && u32::from(block[114]) >= refresh);
            assert_eq!(block[75], 0x10);
        }
    }
}
