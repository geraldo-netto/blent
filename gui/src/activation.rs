//! Window activation consumes requests without creating another application owner.
use eframe::egui;

pub(crate) fn show(context: &egui::Context) {
    context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    context.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    context.send_viewport_cmd(egui::ViewportCommand::Focus);
}

#[cfg(target_os = "linux")]
pub(crate) struct Window<A> {
    pub(crate) app: A,
    pub(crate) instance: std::rc::Rc<crate::platform::activation::Instance>,
}

#[cfg(target_os = "linux")]
impl<A: eframe::App> eframe::App for Window<A> {
    fn update(&mut self, context: &egui::Context, frame: &mut eframe::Frame) {
        poll(context, &self.instance);
        self.app.update(context, frame);
    }
}

#[cfg(target_os = "linux")]
fn poll(context: &egui::Context, instance: &crate::platform::activation::Instance) {
    if instance.requested().unwrap_or(false) {
        show(context);
    }
    context.request_repaint_after(std::time::Duration::from_millis(250));
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn t1449_activation_restores_original_viewport_without_touching_resources() {
        let root = tempfile::tempdir().unwrap();
        let name = root.path().as_os_str().as_encoded_bytes();
        crate::platform::activation::run_at(name, |instance| {
            crate::platform::activation::run_at(name, |_| panic!("duplicate window"))?;
            let context = egui::Context::default();
            let output = context.run(Default::default(), |context| poll(context, &instance));
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert!(commands.contains(&egui::ViewportCommand::Visible(true)));
            assert!(commands.contains(&egui::ViewportCommand::Minimized(false)));
            assert!(commands.contains(&egui::ViewportCommand::Focus));
            let output = context.run(Default::default(), |context| poll(context, &instance));
            assert!(output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .is_empty());
            Ok(())
        })
        .unwrap();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests {
    use super::*;

    struct Probe;
    impl eframe::App for Probe {
        fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn native_window() {
        use winit::platform::x11::EventLoopBuilderExtX11;
        crate::platform::activation::run(|instance| {
            crate::platform::activation::run(|_| panic!("duplicate"))?;
            let options = eframe::NativeOptions {
                event_loop_builder: Some(Box::new(|builder| {
                    builder.with_x11().with_any_thread(true);
                })),
                ..Default::default()
            };
            eframe::run_native(
                "T1449 private window",
                options,
                Box::new(move |_| {
                    Ok(Box::new(Window {
                        app: Probe,
                        instance,
                    }))
                }),
            )
        })
        .unwrap();
        crate::platform::activation::run(|_| Ok(())).unwrap();
    }

    #[test]
    fn t1449_native_window_on_private_display() {
        if std::env::var_os("BLENT_T1449_PRIVATE").is_some() {
            native_window();
            return;
        }
        let root = tempfile::tempdir().unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = std::process::Command::new("xvfb-run")
            .args(["-a", "-s", "-screen 0 800x600x24 -nolisten tcp"])
            .args(["dbus-run-session", "--"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "activation::native_tests::t1449_native_window_on_private_display",
            ])
            .env("BLENT_T1449_PRIVATE", "1")
            .env("XDG_RUNTIME_DIR", root.path())
            .env("WINIT_UNIX_BACKEND", "x11")
            .env_remove("WAYLAND_DISPLAY")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
