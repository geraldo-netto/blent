//! Linux startup placement. X11 observations are read once the window geometry
//! exists; Wayland owns positioning. The window ID is borrowed only during
//! App startup; eframe keeps that window alive until App teardown.
use crate::window_placement::{center, Area};
use eframe::egui;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use x11rb::{
    connection::Connection,
    protocol::{
        randr::ConnectionExt as _,
        xproto::{AtomEnum, ConnectionExt as _},
    },
};

type Error = Box<dyn std::error::Error>;

#[derive(Clone, Copy)]
struct Target {
    screen: usize,
    window: u32,
}

#[derive(Default)]
pub(crate) struct Initial {
    target: Option<Target>,
}
impl Initial {
    pub fn new(context: &eframe::CreationContext<'_>) -> Self {
        Self {
            target: target(context),
        }
    }

    pub fn apply(&mut self, context: &egui::Context) {
        let Some(target) = self.target else { return };
        match position(target, context.pixels_per_point() as f64) {
            Ok(None) => context.request_repaint_after(std::time::Duration::from_millis(20)),
            Ok(Some(position)) => {
                self.target = None;
                context.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position.into()));
            }
            Err(error) => {
                self.target = None;
                eprintln!("Initial window placement unavailable: {error}");
            }
        }
    }
}

fn target(context: &eframe::CreationContext<'_>) -> Option<Target> {
    let screen = screen(context.display_handle().ok()?.as_raw())?;
    let window = window_id(context.window_handle().ok()?.as_raw())?;
    Some(Target { screen, window })
}

fn window_id(handle: RawWindowHandle) -> Option<u32> {
    match handle {
        RawWindowHandle::Xlib(handle) => u32::try_from(handle.window).ok().filter(|id| *id != 0),
        RawWindowHandle::Xcb(handle) => Some(handle.window.get()),
        _ => None,
    }
}

fn screen(display: RawDisplayHandle) -> Option<usize> {
    let index = match display {
        RawDisplayHandle::Xlib(handle) => handle.screen,
        RawDisplayHandle::Xcb(handle) => handle.screen,
        _ => return None,
    };
    usize::try_from(index).ok()
}

/// Own each read-only connection through its replies. No socket or reply cookie
/// survives this call, including mapping waits, rejection and partial failure.
fn position(target: Target, scale: f64) -> Result<Option<[f32; 2]>, Error> {
    let (connection, _) = x11rb::connect(None)?;
    let root = connection
        .setup()
        .roots
        .get(target.screen)
        .ok_or("X11 screen unavailable")?
        .root;
    let attributes = connection.get_window_attributes(target.window)?.reply()?;
    if attributes.map_state != x11rb::protocol::xproto::MapState::VIEWABLE {
        return Ok(None);
    }
    let size = outer_size(&connection, target.window)?;
    let area = area_on(&connection, root)?;
    center(area, size.map(|value| value / scale), scale)
        .map(Some)
        .ok_or_else(|| "invalid native window geometry or scale".into())
}

fn outer_size(connection: &impl Connection, window: u32) -> Result<[f64; 2], Error> {
    let geometry = connection.get_geometry(window)?.reply()?;
    let extents = property(connection, window, b"_NET_FRAME_EXTENTS", 0, 4)?;
    let [left, right, top, bottom] = extents.as_slice() else {
        return Ok([geometry.width as f64, geometry.height as f64]);
    };
    Ok([
        geometry.width as f64 + *left as f64 + *right as f64,
        geometry.height as f64 + *top as f64 + *bottom as f64,
    ])
}

fn area_on(connection: &impl Connection, root: u32) -> Result<Area, Error> {
    let monitors = connection.randr_get_monitors(root, false)?.reply()?;
    let monitor = monitors
        .monitors
        .iter()
        .find(|monitor| monitor.primary)
        .ok_or("no primary monitor declared by X11")?;
    let area = Area {
        origin: [monitor.x as f64, monitor.y as f64],
        size: [monitor.width as f64, monitor.height as f64],
    };
    if !area.valid() {
        return Err("invalid primary monitor bounds".into());
    }
    let usable = work_area(connection, root)?.and_then(|work| area.intersect(work));
    Ok(usable.unwrap_or(area))
}

fn property(
    connection: &impl Connection,
    root: u32,
    name: &[u8],
    offset: u32,
    count: u32,
) -> Result<Vec<u32>, Error> {
    let atom = connection.intern_atom(true, name)?.reply()?.atom;
    if atom == 0 {
        return Ok(Vec::new());
    }
    let value = connection
        .get_property(false, root, atom, AtomEnum::CARDINAL, offset, count)?
        .reply()?;
    Ok(value
        .value32()
        .map(|values| values.collect())
        .unwrap_or_default())
}

fn work_area(connection: &impl Connection, root: u32) -> Result<Option<Area>, Error> {
    let desktop = property(connection, root, b"_NET_CURRENT_DESKTOP", 0, 1)?;
    let Some(offset) = desktop.first().copied().unwrap_or(0).checked_mul(4) else {
        return Ok(None);
    };
    let values = property(connection, root, b"_NET_WORKAREA", offset, 4)?;
    let [x, y, width, height] = values.as_slice() else {
        return Ok(None);
    };
    Ok(Some(Area {
        origin: [*x as i32 as f64, *y as i32 as f64],
        size: [*width as f64, *height as f64],
    }))
}

#[cfg(test)]
pub(crate) mod tests;
