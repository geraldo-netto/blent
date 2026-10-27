//! Stateless geometry in physical desktop pixels; no native handles or storage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Area {
    pub origin: [f64; 2],
    pub size: [f64; 2],
}
impl Area {
    pub fn valid(self) -> bool {
        self.origin.iter().all(|value| value.is_finite())
            && self
                .size
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        if !self.valid() || !other.valid() {
            return None;
        }
        let origin = std::array::from_fn(|axis| self.origin[axis].max(other.origin[axis]));
        let size = std::array::from_fn(|axis| {
            (self.origin[axis] + self.size[axis]).min(other.origin[axis] + other.size[axis])
                - origin[axis]
        });
        let area = Self { origin, size };
        area.valid().then_some(area)
    }
}

/// Convert the physical target to UI points using this window's current scale.
/// Oversized windows keep their top/left controls on the selected monitor.
pub(crate) fn center(area: Area, window: [f64; 2], scale: f64) -> Option<[f32; 2]> {
    if !area.valid() {
        return None;
    }
    let physical = scaled_window(window, scale)?;
    let position = std::array::from_fn(|axis| {
        ((area.origin[axis] + (area.size[axis] - physical[axis]).max(0.0) / 2.0) / scale) as f32
    });
    position
        .iter()
        .all(|value| value.is_finite())
        .then_some(position)
}

fn scaled_window(window: [f64; 2], scale: f64) -> Option<[f64; 2]> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let physical = window.map(|value| value * scale);
    physical
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
        .then_some(physical)
}

#[cfg(test)]
mod tests;
