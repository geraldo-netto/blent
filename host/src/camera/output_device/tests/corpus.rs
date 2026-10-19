//! Bounded invalid/boundary properties against independent profile/fraction oracles.
use super::*;

fn admitted(width: u32, height: u32, fps: u32) -> bool {
    (160..=1920).contains(&width)
        && width.is_multiple_of(2)
        && (120..=1080).contains(&height)
        && height.is_multiple_of(2)
        && (5..=30).contains(&fps)
}
#[test]
fn t1143_invalid_and_out_of_bounds_profiles_never_form_a_native_request() {
    for width in [0, 1, 159, 160, 161, 1919, 1920, 1921, u32::MAX] {
        for height in [0, 1, 119, 120, 121, 1079, 1080, 1081, u32::MAX] {
            for fps in [0, 4, 5, 30, 31, u32::MAX] {
                let result = Spec::from_profile(&profile(width, height, fps));
                assert_eq!(result.is_ok(), admitted(width, height, fps));
                if let Ok(image) = result {
                    assert_eq!(image.dimensions(), (width, height));
                    assert_eq!(image.fps(), fps);
                    assert_eq!(
                        u64::from(image.frame_bytes()),
                        u64::from(width) * u64::from(height) * 3 / 2
                    );
                }
            }
        }
    }
}

fn image_fields() -> [(u32, ImageChange); 7] {
    [
        (2, |format, value| format.kind = value),
        (320, |format, value| format.image.pixels.width = value),
        (240, |format, value| format.image.pixels.height = value),
        (u32::from_le_bytes(*b"YU12"), |format, value| {
            format.image.pixels.format = value
        }),
        (1, |format, value| format.image.pixels.field = value),
        (320, |format, value| format.image.pixels.stride = value),
        (115200, |format, value| format.image.pixels.bytes = value),
    ]
}
#[test]
fn t1143_adjusted_image_fields_reject_before_rate_or_writer_admission() {
    let file = File::open("/dev/null").unwrap();
    for (expected, change) in image_fields() {
        for value in [0, 1, 2, u32::MAX, expected, expected.saturating_add(1)] {
            let control = Control {
                image: Some((change, value)),
                ..Control::default()
            };
            assert_eq!(
                configure_with(&control, file.try_clone().unwrap(), spec()).is_ok(),
                value == expected
            );
            assert_eq!(
                control.events.borrow().len(),
                if value == expected { 3 } else { 1 }
            );
        }
    }
}

fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}
fn exact(period: abi::Fraction) -> bool {
    if period.numerator == 0 || period.denominator == 0 {
        return false;
    }
    let divisor = gcd(period.numerator, period.denominator);
    (period.numerator / divisor, period.denominator / divisor) == (1, 30)
}

#[test]
fn t1143_queried_rate_boundaries_and_unknown_types_fail_closed() {
    let file = File::open("/dev/null").unwrap();
    for kind in [0, 1, 2, u32::MAX] {
        for numerator in [0, 1, 2, u32::MAX] {
            for denominator in [0, 1, 5, 30, u32::MAX] {
                let mut control = Control {
                    rate_kind: kind,
                    ..Control::default()
                };
                control.rate.period = abi::Fraction {
                    numerator,
                    denominator,
                };
                let expected = kind == 2 && numerator > 0 && denominator > 0;
                assert_eq!(
                    configure_with(&control, file.try_clone().unwrap(), spec()).is_ok(),
                    expected
                );
            }
        }
    }
}
fn rate_fields() -> [RateChange; 3] {
    [
        |rate, value| rate.kind = value,
        |rate, value| rate.timing.output.period.numerator = value,
        |rate, value| rate.timing.output.period.denominator = value,
    ]
}
#[test]
fn t1143_set_rate_readback_uses_exact_fraction_and_bounded_wide_arithmetic() {
    let file = File::open("/dev/null").unwrap();
    for change in rate_fields() {
        for value in [0, 1, 2, 5, 30, u32::MAX] {
            let control = Control {
                change: Some((change, value)),
                ..Control::default()
            };
            let mut expected = abi::Parameters::new(30);
            change(&mut expected, value);
            let accepted = expected.kind == 2 && exact(unsafe { expected.timing.output.period });
            assert_eq!(
                configure_with(&control, file.try_clone().unwrap(), spec()).is_ok(),
                accepted
            );
        }
    }
}
