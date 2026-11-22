//! Original real window remains admitted across close, then tray-style activation.
use super::*;

struct Probe {
    app: App,
    stage: Rc<Cell<u8>>,
    drops: Rc<Cell<usize>>,
    endpoint: Vec<u8>,
}

impl eframe::App for Probe {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        self.app.show_window(context);
        match self.stage.get() {
            0 => {
                self.stage.set(1);
                context.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            1 if context.input(|input| input.viewport().close_requested()) => {
                self.stage.set(2);
                crate::platform::activation::run_at(&self.endpoint, |_| {
                    panic!("T1669 admitted a second media owner")
                })
                .unwrap();
            }
            2 => {
                assert_eq!(self.drops.get(), 0);
                assert_eq!(
                    self.app.camera.backend.as_ref().unwrap().state(),
                    CameraState::Streaming
                );
                self.app.camera.stop();
                self.stage.set(3);
                context.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }
}

fn run_window() {
    use winit::platform::x11::EventLoopBuilderExtX11;
    let (root, mut app) = settings_test_app(Tab::Overview);
    app.cfg = app.saved_cfg.clone();
    let drops = Rc::new(Cell::new(0));
    app.camera.backend = Some(Box::new(Camera(drops.clone(), CameraState::Streaming)));
    let stage = Rc::new(Cell::new(0));
    let endpoint = root.path().as_os_str().as_encoded_bytes().to_vec();
    crate::platform::activation::run_at(&endpoint.clone(), |instance| {
        let options = eframe::NativeOptions {
            event_loop_builder: Some(Box::new(|builder| {
                builder.with_x11().with_any_thread(true);
            })),
            ..Default::default()
        };
        eframe::run_native(
            "T1669 private media window",
            options,
            Box::new(|cc| {
                let probe = Probe {
                    app,
                    stage: stage.clone(),
                    drops: drops.clone(),
                    endpoint,
                };
                Ok(Box::new(crate::activation::Window::new(
                    probe,
                    &instance,
                    &cc.egui_ctx,
                )?))
            }),
        )
    })
    .unwrap();
    assert_eq!(
        stage.get(),
        3,
        "T1669 close retired the original window before reactivation"
    );
    assert_eq!(drops.get(), 1);
}

#[test]
fn t1669_native_close_reactivate_and_explicit_stop_on_private_display() {
    if std::env::var_os("BLENT_T1669_PRIVATE").is_some() {
        run_window();
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = std::process::Command::new("xvfb-run")
        .args(["-a", "-s", "-screen 0 1040x740x24 -nolisten tcp", "dbus-run-session", "--"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "tests::close_tests::native::t1669_native_close_reactivate_and_explicit_stop_on_private_display"])
        .env("BLENT_T1669_PRIVATE", "1").env("XDG_RUNTIME_DIR", root.path())
        .env("WINIT_UNIX_BACKEND", "x11").env_remove("WAYLAND_DISPLAY")
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Startup(crate::App);
impl eframe::App for Startup {
    fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        self.0.show_window(context);
        assert!(self.0.cfg == self.0.saved_cfg);
        assert!(self.0.close_intent == crate::window_lifecycle::CloseIntent::None);
        if self.0.update.lock().unwrap().is_some() { context.send_viewport_cmd(egui::ViewportCommand::Close); }
        else { context.request_repaint_after(std::time::Duration::from_millis(10)); }
    }
}

fn startup_window() {
    use winit::platform::x11::EventLoopBuilderExtX11;
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(|builder| { builder.with_x11().with_any_thread(true); })),
        ..Default::default()
    };
    eframe::run_native("T1783 private startup", options, Box::new(|cc| {
        Ok(Box::new(Startup(crate::App::new(cc, "Fixture scheduling".into()))))
    })).unwrap();
}

#[test]
fn t1783_native_clean_startup_closes_without_a_prompt() {
    if std::env::var_os("BLENT_T1783_PRIVATE").is_some() { startup_window(); return; }
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::create_dir(root.path().join("blent")).unwrap();
    std::fs::write(root.path().join("blent/config.toml"), "check_updates = true").unwrap();
    let stub = root.path().join("curl");
    std::fs::write(&stub, "#!/bin/sh\nprintf '%s' '{\"tag_name\":\"v999.0.0\"}'\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = std::process::Command::new("xvfb-run")
        .args(["-a", "-s", "-screen 0 1040x740x24 -nolisten tcp", "dbus-run-session", "--"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "tests::close_tests::native::t1783_native_clean_startup_closes_without_a_prompt"])
        .env("BLENT_T1783_PRIVATE", "1").env("XDG_RUNTIME_DIR", root.path())
        .env("PATH", format!("{}:{}", root.path().display(), std::env::var("PATH").unwrap()))
        .env("XDG_CONFIG_HOME", root.path()).env("WINIT_UNIX_BACKEND", "x11")
        .env_remove("WAYLAND_DISPLAY").output().unwrap();
    assert!(output.status.success(), "{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}
