//! T1315: actual eframe placement on an owned Xvfb, never the user's display.
use super::*;
use x11rb::{
    connection::Connection,
    protocol::{
        randr::{ConnectionExt as _, MonitorInfo},
        xproto::ConnectionExt as _,
    },
};

fn primary_monitor() -> x11rb::rust_connection::RustConnection {
    let (connection, screen) = x11rb::connect(None).unwrap();
    let root = connection.setup().roots[screen].root;
    let name = connection
        .intern_atom(false, b"T1315-primary")
        .unwrap()
        .reply()
        .unwrap()
        .atom;
    connection
        .randr_set_monitor(
            root,
            MonitorInfo {
                name,
                primary: true,
                automatic: false,
                x: 1200,
                y: 700,
                width: 1600,
                height: 1000,
                width_in_millimeters: 400,
                height_in_millimeters: 250,
                outputs: vec![],
            },
        )
        .unwrap()
        .check()
        .unwrap();
    connection.flush().unwrap();
    let monitors = connection
        .randr_get_monitors(root, false)
        .unwrap()
        .reply()
        .unwrap();
    assert!(monitors.monitors.iter().any(|m| m.primary && m.x == 1200));
    connection
}

struct Probe {
    placement: platform::placement::Initial,
    frames: usize,
    observed: Arc<Mutex<Option<egui::Rect>>>,
}
impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.placement.apply(ctx);
        self.frames += 1;
        if self.frames >= 100 {
            *self.observed.lock().unwrap() = ctx.input(|i| i.viewport().outer_rect);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(20));
    }
}

fn native_probe() {
    use winit::platform::x11::EventLoopBuilderExtX11;
    let connection = primary_monitor();
    platform::placement::tests::native_contracts(&connection);
    platform::placement::tests::native_mapping_lifetime(&connection);
    platform::placement::tests::native_layouts(&connection);
    let observed = Arc::new(Mutex::new(None));
    let output = observed.clone();
    let mut options = native_options();
    options.event_loop_builder = Some(Box::new(|builder| {
        builder.with_x11().with_any_thread(true);
    }));
    eframe::run_native(
        "T1315 placement",
        options,
        Box::new(move |context| {
            Ok(Box::new(Probe {
                placement: platform::placement::Initial::new(context),
                frames: 0,
                observed: output,
            }))
        }),
    )
    .unwrap();
    let rect = observed.lock().unwrap().unwrap();
    let scale: f32 = std::env::var("WINIT_X11_SCALE_FACTOR")
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        (rect.center().x * scale - 2000.0).abs() <= 1.0,
        "T1315 horizontal center: {rect:?}"
    );
    assert!(
        (rect.center().y * scale - 1200.0).abs() <= 1.0,
        "T1315 vertical center: {rect:?}"
    );
}

#[test]
fn t1315_native_primary_monitor_offset() {
    if std::env::var_os("BLENT_T1315_PRIVATE_DISPLAY").is_some() {
        native_probe();
        return;
    }
    for scale in ["1", "1.25"] {
        for manager in ["none", "metacity"] {
            let output = Command::new("xvfb-run")
                .args(["-a", "-s", "-screen 0 3000x1800x24 -nolisten tcp"])
                .args(["dbus-run-session", "--", "python3"])
                .arg(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/placement_manager.py"),
                )
                .arg(manager)
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "placement_tests::t1315_native_primary_monitor_offset",
                    "--nocapture",
                ])
                .env("BLENT_T1315_PRIVATE_DISPLAY", "1")
                .env("WINIT_UNIX_BACKEND", "x11")
                .env("WINIT_X11_SCALE_FACTOR", scale)
                .env_remove("WAYLAND_DISPLAY")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
