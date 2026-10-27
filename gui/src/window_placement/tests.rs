use super::*;

#[test]
fn t1315_dynamic_origins_sizes_and_scales() {
    for origin in [
        [0.0, 0.0],
        [1280.0, 1080.0],
        [-3840.0, -2160.0],
        [9100.0, -370.0],
    ] {
        for size in [[1920.0, 1080.0], [3840.0, 2160.0], [1600.0, 2560.0]] {
            for scale in [0.75, 1.0, 1.25, 1.5, 2.0, 3.0] {
                let window = [240.0, 180.0];
                let position = center(Area { origin, size }, window, scale).unwrap();
                for axis in 0..2 {
                    let midpoint = (position[axis] as f64 + window[axis] / 2.0) * scale;
                    assert!((midpoint - origin[axis] - size[axis] / 2.0).abs() < 0.01);
                }
            }
        }
    }
}

#[test]
fn t1315_invalid_geometry_rejects_without_position() {
    let valid = Area {
        origin: [0.0, 0.0],
        size: [1920.0, 1080.0],
    };
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0] {
        assert!(center(valid, [100.0, 100.0], value).is_none());
        assert!(center(valid, [value, 100.0], 1.0).is_none());
        assert!(center(
            Area {
                size: [100.0, value],
                ..valid
            },
            [100.0, 100.0],
            1.0
        )
        .is_none());
    }
    assert!(center(
        Area {
            origin: [f64::NAN, 0.0],
            ..valid
        },
        [100.0, 100.0],
        1.0
    )
    .is_none());
    assert!(center(valid, [f64::MAX, 1.0], 2.0).is_none());
    assert!(center(
        Area {
            origin: [f64::MAX, 0.0],
            ..valid
        },
        [1.0, 1.0],
        1.0
    )
    .is_none());
}

#[test]
fn t1315_oversized_windows_and_usable_intersections() {
    let monitor = Area {
        origin: [-1600.0, 200.0],
        size: [1600.0, 900.0],
    };
    assert_eq!(
        center(monitor, [2000.0, 1000.0], 1.0),
        Some([-1600.0, 200.0])
    );
    let work = Area {
        origin: [-1600.0, 230.0],
        size: [4000.0, 2000.0],
    };
    assert_eq!(
        monitor.intersect(work),
        Some(Area {
            origin: [-1600.0, 230.0],
            size: [1600.0, 870.0]
        })
    );
    assert_eq!(
        monitor.intersect(Area {
            origin: [20.0, 20.0],
            ..work
        }),
        None
    );
    assert_eq!(
        monitor.intersect(Area {
            size: [0.0, 0.0],
            ..work
        }),
        None
    );
    assert_eq!(
        Area {
            size: [0.0, 0.0],
            ..monitor
        }
        .intersect(work),
        None
    );
}
