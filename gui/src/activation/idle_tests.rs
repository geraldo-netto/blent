//! T1550/T1551: actual private-window callbacks, with the old timers as controls.
use super::*;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

struct Probe {
    frames: Arc<AtomicUsize>,
    started: Instant,
    timer: Option<Duration>,
    overview: Option<crate::App>,
}
impl eframe::App for Probe {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        let age = self.started.elapsed();
        if age >= Duration::from_millis(500) && age < Duration::from_secs(2) {
            self.frames.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(app) = &mut self.overview {
            app.show_window(context);
        }
        if let Some(timer) = self.timer {
            context.request_repaint_after(timer);
        }
    }
}

fn measured_window(mode: &str) {
    use winit::platform::x11::EventLoopBuilderExtX11;
    let (_root, mut app) = crate::tests::settings_test_app(crate::Tab::Overview);
    app.selected = Some(blent_config::tablets::Identity::Adb("T1551-tablet".into()));
    app.status.lock().unwrap().daemon_running = true;
    app.status.lock().unwrap().display = Some(blent_config::display_control::Snapshot {
        owner: "ab".repeat(32),
        assignments: vec![blent_config::display_control::Assignment {
            target: blent_config::display_control::Target::Adb("T1551-tablet".into()),
            lease: blent_config::display_control::Lease {
                slot: 0,
                generation: 1,
            },
            phase: blent_config::display_control::Phase::Assigned,
        }],
    });
    let timer = match mode {
        "old-wrapper" => Some(Duration::from_millis(250)),
        "old-overview" => Some(Duration::from_millis(100)),
        _ => None,
    };
    let frames = Arc::new(AtomicUsize::new(0));
    let measured = frames.clone();
    let mut close = None;
    crate::platform::activation::run(|instance| {
        let options = eframe::NativeOptions {
            event_loop_builder: Some(Box::new(|builder| {
                builder.with_x11().with_any_thread(true);
            })),
            ..Default::default()
        };
        eframe::run_native(
            "Blent private idle measurement",
            options,
            Box::new(|cc| {
                let context = cc.egui_ctx.clone();
                close = Some(std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(2));
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }));
                Ok(Box::new(Window::new(
                    Probe {
                        frames: measured,
                        started: Instant::now(),
                        timer,
                        overview: mode.contains("overview").then_some(app),
                    },
                    &instance,
                    &cc.egui_ctx,
                )?))
            }),
        )
    })
    .unwrap();
    close.unwrap().join().unwrap();
    let count = frames.load(Ordering::Relaxed);
    println!("T1550/T1551 {mode}: {count} update callbacks in the 1.5-second idle observation");
    if mode.starts_with("old-") {
        assert!(count >= 4, "fault control did not reproduce timed frames");
    } else {
        assert!(count <= 2, "idle window retained repeated wakeups");
    }
}

#[test]
fn t1550_t1551_private_window_idle_callback_measurements() {
    if let Ok(mode) = std::env::var("BLENT_T1550_IDLE") {
        measured_window(&mode);
        return;
    }
    for mode in [
        "old-wrapper",
        "fixed-wrapper",
    ] {
        let root = tempfile::tempdir().unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = std::process::Command::new("xvfb-run")
            .args([
                "-a",
                "-s",
                "-screen 0 1280x1024x24 -nolisten tcp",
                "dbus-run-session",
                "--",
            ])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "activation::idle_tests::t1550_t1551_private_window_idle_callback_measurements",
                "--nocapture",
            ])
            .env("BLENT_T1550_IDLE", mode)
            .env("XDG_RUNTIME_DIR", root.path())
            .env("HOME", root.path())
            .env("XDG_CONFIG_HOME", root.path())
            .env("WINIT_UNIX_BACKEND", "x11")
            .env("LIBGL_ALWAYS_SOFTWARE", "1")
            .env_remove("WAYLAND_DISPLAY")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
}
