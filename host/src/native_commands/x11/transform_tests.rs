use super::transform::matrix;
use super::*;
fn point(matrix: [f32; 9], x: f32, y: f32) -> (f32, f32) {
    (
        matrix[0] * x + matrix[1] * y + matrix[2],
        matrix[3] * x + matrix[4] * y + matrix[5],
    )
}
#[test]
fn t1641_all_orientations_map_each_corner_into_its_rectangle() {
    // Independent corner oracle: normalized clockwise screen coordinates.
    let turns = [Turn::Zero, Turn::Left, Turn::Half, Turn::Right];
    let expected = [
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
        [(1.0, 0.0), (1.0, 1.0), (0.0, 0.0), (0.0, 1.0)],
        [(1.0, 1.0), (0.0, 1.0), (1.0, 0.0), (0.0, 0.0)],
        [(0.0, 1.0), (0.0, 0.0), (1.0, 1.0), (1.0, 0.0)],
    ];
    for (turn, corners) in turns.into_iter().zip(expected) {
        for reflect_x in [false, true] {
            for reflect_y in [false, true] {
                check_corners(
                    Orientation {
                        turn,
                        reflect_x,
                        reflect_y,
                    },
                    corners,
                );
            }
        }
    }
}
fn check_corners(orientation: Orientation, corners: [(f32, f32); 4]) {
    let result = matrix(Geometry {
        canvas: (8, 8),
        rectangle: (1, 2, 2, 4),
        orientation,
    })
    .unwrap();
    for ((x, y), (mut expected_x, mut expected_y)) in
        [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
            .into_iter()
            .zip(corners)
    {
        if orientation.reflect_x {
            expected_x = 1.0 - expected_x;
        }
        if orientation.reflect_y {
            expected_y = 1.0 - expected_y;
        }
        assert_eq!(
            point(result, x, y),
            (0.125 + 0.25 * expected_x, 0.25 + 0.5 * expected_y)
        );
    }
    assert_eq!(&result[6..], &[0.0, 0.0, 1.0]);
}
#[test]
fn t1641_geometry_and_rotation_domains_reject_invalid_values() {
    let orientation = Orientation {
        turn: Turn::Zero,
        reflect_x: false,
        reflect_y: false,
    };
    for (canvas, rectangle) in [
        ((0, 8), (1, 2, 2, 4)),
        ((8, 0), (1, 2, 2, 4)),
        ((8, 8), (1, 2, 0, 4)),
        ((8, 8), (1, 2, 2, 0)),
    ] {
        assert!(matrix(Geometry {
            canvas,
            rectangle,
            orientation
        })
        .is_none());
    }
    for bits in 0..=u16::MAX {
        let expected = bits & !63 == 0 && matches!(bits & 15, 1 | 2 | 4 | 8);
        assert_eq!(super::orientation(bits.into()).is_ok(), expected);
    }
    let result = matrix(Geometry {
        canvas: (1, 1),
        rectangle: (i16::MIN, i16::MAX, u16::MAX, u16::MAX),
        orientation,
    })
    .unwrap();
    assert!(result.iter().all(|value| value.is_finite()));
}
