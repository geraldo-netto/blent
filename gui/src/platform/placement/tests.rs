use super::*;

#[test]
fn t1315_backend_selection_never_treats_wayland_as_x11() {
    use raw_window_handle::{WaylandDisplayHandle, XcbDisplayHandle, XlibDisplayHandle};
    assert_eq!(
        screen(RawDisplayHandle::Xlib(XlibDisplayHandle::new(None, 2))),
        Some(2)
    );
    assert_eq!(
        screen(RawDisplayHandle::Xcb(XcbDisplayHandle::new(None, 3))),
        Some(3)
    );
    assert_eq!(
        screen(RawDisplayHandle::Xlib(XlibDisplayHandle::new(None, -1))),
        None
    );
    let wayland = WaylandDisplayHandle::new(std::ptr::NonNull::dangling());
    assert_eq!(screen(RawDisplayHandle::Wayland(wayland)), None);
    let context = eframe::CreationContext::_new_kittest(egui::Context::default());
    assert!(Initial::new(&context).target.is_none());
}

#[test]
fn t1315_default_placement_submits_no_commands() {
    let context = egui::Context::default();
    let output = context.run(egui::RawInput::default(), |context| {
        Initial::default().apply(context)
    });
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .is_empty());
}

#[test]
fn t1315_window_ids_reject_foreign_and_invalid_handles() {
    use raw_window_handle::{WaylandWindowHandle, XcbWindowHandle, XlibWindowHandle};
    assert_eq!(
        window_id(RawWindowHandle::Xlib(XlibWindowHandle::new(0))),
        None
    );
    assert_eq!(
        window_id(RawWindowHandle::Xlib(XlibWindowHandle::new(23))),
        Some(23)
    );
    assert_eq!(
        window_id(RawWindowHandle::Xcb(XcbWindowHandle::new(
            std::num::NonZeroU32::new(31).unwrap()
        ))),
        Some(31)
    );
    assert_eq!(
        window_id(RawWindowHandle::Wayland(WaylandWindowHandle::new(
            std::ptr::NonNull::dangling()
        ))),
        None
    );
}

fn cardinal(connection: &impl Connection, root: u32, name: &[u8], values: &[u32]) -> u32 {
    use x11rb::wrapper::ConnectionExt as _;
    let atom = connection
        .intern_atom(false, name)
        .unwrap()
        .reply()
        .unwrap()
        .atom;
    connection
        .change_property32(
            x11rb::protocol::xproto::PropMode::REPLACE,
            root,
            atom,
            AtomEnum::CARDINAL,
            values,
        )
        .unwrap()
        .check()
        .unwrap();
    atom
}

pub(crate) fn native_contracts(connection: &impl Connection) {
    let root = connection.setup().roots[0].root;
    let expected = Area {
        origin: [1200.0, 700.0],
        size: [1600.0, 1000.0],
    };
    assert_eq!(area_on(connection, root).unwrap(), expected);
    assert!(position(
        Target {
            screen: usize::MAX,
            window: root
        },
        1.0
    )
    .is_err());
    assert!(area_on(connection, 0).is_err());
    let desktop = cardinal(connection, root, b"_NET_CURRENT_DESKTOP", &[1]);
    let work = cardinal(
        connection,
        root,
        b"_NET_WORKAREA",
        &[0, 0, 3000, 1800, 0, 730, 3000, 1070],
    );
    assert_eq!(
        area_on(connection, root).unwrap(),
        Area {
            origin: [1200.0, 730.0],
            size: [1600.0, 970.0]
        }
    );
    cardinal(connection, root, b"_NET_CURRENT_DESKTOP", &[u32::MAX]);
    assert_eq!(area_on(connection, root).unwrap(), expected);
    connection
        .delete_property(root, desktop)
        .unwrap()
        .check()
        .unwrap();
    connection
        .delete_property(root, work)
        .unwrap()
        .check()
        .unwrap();
}

pub(crate) fn native_layouts(connection: &impl Connection) {
    use x11rb::protocol::randr::MonitorInfo;
    let root = connection.setup().roots[0].root;
    let original = connection
        .randr_get_monitors(root, false)
        .unwrap()
        .reply()
        .unwrap()
        .monitors
        .into_iter()
        .find(|m| m.primary)
        .unwrap();
    let mut names = Vec::new();
    for index in 0..23 {
        let name = connection
            .intern_atom(false, format!("T1315-extra-{index}").as_bytes())
            .unwrap()
            .reply()
            .unwrap()
            .atom;
        names.push(name);
        connection
            .randr_set_monitor(
                root,
                MonitorInfo {
                    name,
                    primary: false,
                    automatic: false,
                    x: -32000 + index * 1300,
                    y: -1000 + index * 90,
                    width: 1000,
                    height: 700,
                    width_in_millimeters: 250,
                    height_in_millimeters: 175,
                    outputs: vec![],
                },
            )
            .unwrap()
            .check()
            .unwrap();
        assert_eq!(area_on(connection, root).unwrap().origin, [1200.0, 700.0]);
    }
    for [x, y] in [[0, 0], [-1600, -200], [900, 1000]] {
        connection
            .randr_delete_monitor(root, original.name)
            .unwrap()
            .check()
            .unwrap();
        let mut changed = original.clone();
        changed.x = x;
        changed.y = y;
        connection
            .randr_set_monitor(root, changed)
            .unwrap()
            .check()
            .unwrap();
        assert_eq!(
            area_on(connection, root).unwrap().origin,
            [x as f64, y as f64]
        );
    }
    connection
        .randr_delete_monitor(root, original.name)
        .unwrap()
        .check()
        .unwrap();
    connection
        .randr_set_monitor(root, original)
        .unwrap()
        .check()
        .unwrap();
    for name in names {
        connection
            .randr_delete_monitor(root, name)
            .unwrap()
            .check()
            .unwrap();
    }
}

#[test]
fn t1315_unavailable_screen_is_not_retried_or_positioned() {
    let context = egui::Context::default();
    let mut pending = Initial {
        target: Some(Target {
            screen: usize::MAX,
            window: 0,
        }),
    };
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .outer_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(500.0, 400.0),
    ));
    let output = context.run(input, |context| pending.apply(context));
    assert!(pending.target.is_none());
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .is_empty());
}

pub(crate) fn native_mapping_lifetime(connection: &impl Connection) {
    use x11rb::protocol::xproto::{CreateWindowAux, WindowClass};
    let root = connection.setup().roots[0].root;
    let window = connection.generate_id().unwrap();
    connection
        .create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            100,
            100,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::default(),
        )
        .unwrap()
        .check()
        .unwrap();
    let target = Target { screen: 0, window };
    assert_eq!(position(target, 1.0).unwrap(), None);
    let context = egui::Context::default();
    let mut pending = Initial {
        target: Some(target),
    };
    let _ = context.run(egui::RawInput::default(), |context| pending.apply(context));
    assert!(pending.target.is_some());
    connection.destroy_window(window).unwrap().check().unwrap();
    let _ = context.run(egui::RawInput::default(), |context| pending.apply(context));
    assert!(pending.target.is_none());
    let output = context.run(egui::RawInput::default(), |context| pending.apply(context));
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .is_empty());
}
