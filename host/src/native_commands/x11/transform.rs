//! Normalized tablet coordinates into an oriented output rectangle.
//! All data is owned by the call; fixed-size output, no retained allocation.
#[derive(Clone, Copy)]
pub(super) enum Turn {
    Zero,
    Left,
    Half,
    Right,
}
#[derive(Clone, Copy)]
pub(super) struct Orientation {
    pub turn: Turn,
    pub reflect_x: bool,
    pub reflect_y: bool,
}
pub(super) struct Geometry {
    pub canvas: (u16, u16),
    pub rectangle: (i16, i16, u16, u16),
    pub orientation: Orientation,
}
fn corner(orientation: Orientation, x: i8, y: i8) -> (i8, i8) {
    let (mut x, mut y) = match orientation.turn {
        Turn::Zero => (x, y),
        Turn::Left => (1 - y, x),
        Turn::Half => (1 - x, 1 - y),
        Turn::Right => (y, 1 - x),
    };
    if orientation.reflect_x {
        x = 1 - x;
    }
    if orientation.reflect_y {
        y = 1 - y;
    }
    (x, y)
}
pub(super) fn matrix(geometry: Geometry) -> Option<[f32; 9]> {
    let (canvas_w, canvas_h) = geometry.canvas;
    let (x, y, width, height) = geometry.rectangle;
    if canvas_w == 0 || canvas_h == 0 || width == 0 || height == 0 {
        return None;
    }
    let (w, h) = (
        f32::from(width) / f32::from(canvas_w),
        f32::from(height) / f32::from(canvas_h),
    );
    let origin = corner(geometry.orientation, 0, 0);
    let across = corner(geometry.orientation, 1, 0);
    let down = corner(geometry.orientation, 0, 1);
    Some([
        w * f32::from(across.0 - origin.0),
        w * f32::from(down.0 - origin.0),
        f32::from(x) / f32::from(canvas_w) + w * f32::from(origin.0),
        h * f32::from(across.1 - origin.1),
        h * f32::from(down.1 - origin.1),
        f32::from(y) / f32::from(canvas_h) + h * f32::from(origin.1),
        0.0,
        0.0,
        1.0,
    ])
}
