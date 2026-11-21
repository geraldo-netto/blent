use super::*;
fn capabilities() -> Capabilities {
    Capabilities::decode([
        255, 255, 255, 250, 0, 0, 0, 6, 0, 0, 0, 1, 0, 0, 0, 3, 1, 7, 15, 0,
    ])
    .unwrap()
}
#[test]
fn t1468_command_has_independent_cross_language_byte_oracle() {
    let profile = super::super::CameraProfile::default();
    let encoding = Encoding::from_profile(&profile);
    let mut edited = profile.clone();
    encoding.write_profile(&mut edited);
    assert_eq!(profile, edited);
    let command = Command {
        revision: 1,
        sensor: Sensor {
            exposure: -100,
            metering: Metering::Center,
            noise: Noise::Fast,
            flicker: Flicker::Hz50,
        },
        encoding: Encoding {
            ceiling: 3000,
            floor: 1000,
            adaptive: true,
        },
    };
    assert_eq!(
        command.encode(),
        [0, 0, 0, 0, 0, 0, 0, 1, 255, 156, 1, 2, 2, 11, 184, 3, 232, 1]
    );
    assert_eq!(
        Sensor::default(),
        Sensor {
            exposure: 0,
            metering: Metering::Device,
            noise: Noise::Device,
            flicker: Flicker::Device
        }
    );
}
#[test]
fn t1468_capabilities_and_sensor_bounds_are_independent() {
    let caps = capabilities();
    assert_eq!(caps.exposure_bounds(), (-2.0, 2.0, 1.0 / 3.0));
    for exposure in -400..=400 {
        let sensor = Sensor {
            exposure,
            ..Default::default()
        };
        assert_eq!(sensor.validate().is_ok(), (-300..=300).contains(&exposure));
        assert_eq!(
            caps.accepts(sensor).is_ok(),
            (-200..=200).contains(&exposure)
        );
    }
    let minimal = Capabilities {
        minimum: 0,
        maximum: 0,
        numerator: 0,
        denominator: 1,
        center: false,
        noise_mask: 1,
        flicker_mask: 1,
    };
    assert!(minimal.accepts(Sensor::default()).is_ok());
    for sensor in [
        Sensor {
            metering: Metering::Center,
            ..Default::default()
        },
        Sensor {
            noise: Noise::Fast,
            ..Default::default()
        },
        Sensor {
            flicker: Flicker::Auto,
            ..Default::default()
        },
    ] {
        assert!(minimal.accepts(sensor).is_err());
        assert!(caps.accepts(sensor).is_ok());
    }
}
#[test]
fn t1468_invalid_capability_fields_are_rejected() {
    let valid = [
        255, 255, 255, 250, 0, 0, 0, 6, 0, 0, 0, 1, 0, 0, 0, 3, 1, 7, 15, 0,
    ];
    for (index, value) in [
        (0, 1),
        (4, 255),
        (8, 255),
        (12, 255),
        (15, 0),
        (16, 2),
        (17, 0),
        (17, 8),
        (18, 0),
        (18, 16),
        (19, 1),
    ] {
        let mut bytes = valid;
        bytes[index] = value;
        assert!(Capabilities::decode(bytes).is_err(), "{index}:{value}");
    }
}
