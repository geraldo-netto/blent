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
    activation: crate::platform::activation::Wake,
    appearance: Option<crate::platform::appearance::Watch>,
}

#[cfg(target_os = "linux")]
impl<A> Window<A> {
    pub(crate) fn new(
        app: A,
        instance: &crate::platform::activation::Instance,
        context: &egui::Context,
    ) -> std::io::Result<Self> {
        let wake = context.clone();
        let activation = instance.watch(move || wake.request_repaint())?;
        let wake = context.clone();
        let appearance =
            match crate::platform::appearance::Watch::start(move || wake.request_repaint()) {
                Ok(watch) => Some(watch),
                Err(error) => {
                    crate::workspace_style::refresh_desktop(context, Err(error.to_string()));
                    None
                }
            };
        Ok(Self {
            app,
            activation,
            appearance,
        })
    }
}

#[cfg(target_os = "linux")]
impl<A: eframe::App> eframe::App for Window<A> {
    fn update(&mut self, context: &egui::Context, frame: &mut eframe::Frame) {
        poll(context, &self.activation);
        if let Some(update) = self.appearance.as_ref().and_then(|watch| watch.take()) {
            crate::workspace_style::refresh_desktop(context, update);
        }
        self.app.update(context, frame);
    }
}

#[cfg(target_os = "linux")]
fn poll(context: &egui::Context, activation: &crate::platform::activation::Wake) {
    if activation.requested() {
        show(context);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn t1550_idle_activation_wrapper_does_not_schedule_frames() {
        let root = tempfile::tempdir().unwrap();
        crate::platform::activation::run_at(
            root.path().as_os_str().as_encoded_bytes(),
            |instance| {
                let context = egui::Context::default();
                let wake_context = context.clone();
                let (sent, received) = std::sync::mpsc::channel();
                let activation = instance
                    .watch(move || {
                        wake_context.request_repaint();
                        let _ = sent.send(());
                    })
                    .unwrap();
                for _ in 0..3 {
                    let _ = context.run(Default::default(), |context| poll(context, &activation));
                }
                let output = context.run(Default::default(), |context| poll(context, &activation));
                assert_eq!(
                    output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
                    std::time::Duration::MAX,
                    "T1550 idle activation wrapper keeps waking the GUI"
                );
                assert!(received.try_recv().is_err());
                Ok(())
            },
        )
        .unwrap();
    }

    #[test]
    fn t1449_activation_restores_original_viewport_without_touching_resources() {
        let root = tempfile::tempdir().unwrap();
        let name = root.path().as_os_str().as_encoded_bytes();
        crate::platform::activation::run_at(name, |instance| {
            crate::platform::activation::run_at(name, |_| panic!("duplicate window"))?;
            let context = egui::Context::default();
            let wake_context = context.clone();
            let (sent, received) = std::sync::mpsc::channel();
            let activation = instance
                .watch(move || {
                    wake_context.request_repaint();
                    let _ = sent.send(());
                })
                .unwrap();
            received
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
            let output = context.run(Default::default(), |context| poll(context, &activation));
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert!(commands.contains(&egui::ViewportCommand::Visible(true)));
            assert!(commands.contains(&egui::ViewportCommand::Minimized(false)));
            assert!(commands.contains(&egui::ViewportCommand::Focus));
            let output = context.run(Default::default(), |context| poll(context, &activation));
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

    struct Probe(std::time::Instant);
    impl eframe::App for Probe {
        fn update(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
            assert!(
                std::time::Instant::now() < self.0,
                "T1795 appearance event did not arrive"
            );
            let ready = context
                .data(|data| data.get_temp::<String>(egui::Id::new("desktop-appearance")))
                .is_some();
            if ready {
                context.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                context.request_repaint_after(
                    self.0.saturating_duration_since(std::time::Instant::now()),
                );
            }
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
                Box::new(move |cc| {
                    Ok(Box::new(Window::new(
                        Probe(std::time::Instant::now() + std::time::Duration::from_secs(5)),
                        &instance,
                        &cc.egui_ctx,
                    )?))
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
            .env_remove("GTK_THEME")
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

#[cfg(all(test, target_os = "linux"))]
mod idle_tests;
