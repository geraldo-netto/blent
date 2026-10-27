//! Portable settings/UI; only create_backend selects a native implementation.
use blent_config::camera::{CameraBackend, CameraProfile, CameraState, Lens};
use eframe::egui;
mod connection;

pub(super) struct Panel {
    pub(super) backend: Option<Box<dyn CameraBackend>>,
    error: Option<String>,
    preview: crate::camera_preview::Preview,
    create: fn() -> Result<Box<dyn CameraBackend>, String>,
    transports: blent_config::camera::connection::Transports,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            backend: None,
            error: None,
            preview: Default::default(),
            create: create_backend,
            transports: transport_capabilities(),
        }
    }
}

impl Panel {
    pub(super) fn show(&mut self, ui: &mut egui::Ui, options: &mut CameraProfile) {
        ui.vertical(|ui| {
            self.summary(ui, options);
            super::settings_grid(ui, "camera-basic-grid", |ui| {
                let frame = self.backend.as_ref().and_then(|backend| backend.preview());
                self.preview.show(ui, frame);
                profile(ui, options);
            });
            egui::CollapsingHeader::new("Advanced camera settings")
                .id_salt("camera-advanced")
                .show(ui, |ui| {
                    super::settings_grid(ui, "camera-advanced-grid", |ui| {
                        advanced_profile(ui, options);
                        connection::show(ui, options, self.transports);
                    });
                });
        });
    }

    fn summary(&mut self, ui: &mut egui::Ui, options: &CameraProfile) {
        ui.label(egui::RichText::new("Camera sharing").strong());
        let state = self.state();
        ui.label(state_label(&state));
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }
        self.buttons(ui, options, &state);
        ui.label(egui::RichText::new("Start/Restart camera applies these settings. Apply only saves preferences. Camera changes never restart display sharing.").weak().size(11.0));
        ui.label(egui::RichText::new("Keep this window open while sharing. Select Blent Front or Blent Rear in your video call; only the selected lens is live.").weak().size(11.0));
        ui.add_space(10.0);
    }

    fn state(&self) -> CameraState {
        self.backend
            .as_ref()
            .map_or(CameraState::Stopped, |backend| backend.state())
    }

    fn buttons(&mut self, ui: &mut egui::Ui, options: &CameraProfile, state: &CameraState) {
        let active = state.is_active();
        ui.horizontal_wrapped(|ui| {
            let label = if active {
                "Restart camera"
            } else {
                "Start camera"
            };
            if ui
                .add_enabled(self.transports.available(), egui::Button::new(label))
                .clicked()
            {
                self.error = self.start(options.clone()).err();
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            if ui
                .add_enabled(active, egui::Button::new("Stop camera"))
                .clicked()
            {
                if let Some(backend) = &self.backend {
                    backend.stop();
                }
            }
        });
        if active {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(200));
        }
        if !self.transports.available() {
            ui.label("Camera backend is not implemented on this operating system.");
        }
    }

    fn start(&mut self, options: CameraProfile) -> Result<(), String> {
        options.validate().map_err(|e| e.to_string())?;
        if !self
            .transports
            .supports(&options.target().map_err(|e| e.to_string())?)
        {
            return Err("Selected camera transport is unavailable on this backend.".into());
        }
        if self.backend.is_none() {
            self.backend = Some((self.create)()?);
        }
        self.backend
            .as_ref()
            .unwrap()
            .start(options)
            .map_err(|e| format!("{e:#}"))
    }
}

fn transport_capabilities() -> blent_config::camera::connection::Transports {
    #[cfg(target_os = "linux")]
    return blent::camera::transport_capabilities();
    #[cfg(not(target_os = "linux"))]
    blent_config::camera::connection::Transports::default()
}

fn create_backend() -> Result<Box<dyn CameraBackend>, String> {
    #[cfg(target_os = "linux")]
    return blent::camera_control::Controller::new(blent::camera::run_controlled)
        .map(|controller| Box::new(controller) as Box<dyn CameraBackend>)
        .map_err(|e| e.to_string());
    #[cfg(not(target_os = "linux"))]
    Err("Camera backend is not implemented on this operating system".into())
}

fn state_label(state: &CameraState) -> String {
    match state {
        CameraState::Stopped => "Stopped".into(),
        CameraState::Starting => "Starting camera…".into(),
        CameraState::Paused => {
            "Waiting for host availability. Stop camera cancels pending start or resume.".into()
        }
        CameraState::Waiting => {
            "Waiting for video. Open Blent on the tablet and allow camera access.".into()
        }
        CameraState::Streaming => "Camera video is reaching the host".into(),
        CameraState::Stopping => "Stopping camera…".into(),
        CameraState::Failed(error) => format!("Camera unavailable: {error}"),
    }
}

fn profile(ui: &mut egui::Ui, options: &mut CameraProfile) {
    ui.label("Lens");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut options.lens, Lens::Front, "Front");
        ui.selectable_value(&mut options.lens, Lens::Rear, "Rear");
    });
    ui.end_row();
    resolution(ui, options);
}

fn advanced_profile(ui: &mut egui::Ui, options: &mut CameraProfile) {
    rotation(ui, &mut options.rotation);
    ui.label("Frame rate");
    ui.add(egui::Slider::new(&mut options.fps, 5..=30).suffix(" FPS"));
    ui.end_row();
    ui.label("Camera bitrate ceiling");
    ui.add(egui::Slider::new(&mut options.bitrate, 256..=20000).suffix(" kbit/s"));
    ui.end_row();
    ui.label("Adaptive camera bitrate");
    ui.checkbox(
        &mut options.adaptive_bitrate,
        "Reduce bitrate when the route falls behind",
    );
    ui.end_row();
    ui.label("Camera bitrate floor");
    ui.add_enabled(options.adaptive_bitrate,
        egui::Slider::new(&mut options.min_bitrate, 256..=20000).suffix(" kbit/s"))
        .on_hover_text("Effective floor is capped by the requested ceiling. Lower rates may reduce picture quality.");
    ui.end_row();
    ui.label("Freshness budget");
    ui.add(egui::Slider::new(&mut options.freshness_ms, 50..=2000).suffix(" ms"))
        .on_hover_text("Lower budgets discard backlog sooner, but may freeze more often on a slow route. This is not total camera latency.");
    ui.end_row();
    ui.label("Mirror");
    ui.checkbox(&mut options.mirror, "Flip video horizontally");
    ui.end_row();
    ui.label("Tablet background");
    ui.vertical(|ui| {
        ui.checkbox(&mut options.background, "Continue while hidden or locked");
        ui.label(egui::RichText::new("Start with Blent visible on the tablet. Background sharing keeps its camera service and CPU awake; higher resolution/FPS uses more power.").weak().size(11.0));
    });
    ui.end_row();
}

fn rotation(ui: &mut egui::Ui, rotation: &mut u16) {
    ui.label("Rotate clockwise");
    egui::ComboBox::from_id_salt("camera-rotation")
        .selected_text(format!("{rotation}°"))
        .show_ui(ui, |ui| {
            for degrees in [0, 90, 180, 270] {
                ui.selectable_value(rotation, degrees, format!("{degrees}°"));
            }
        });
    ui.end_row();
}

fn resolution(ui: &mut egui::Ui, options: &mut CameraProfile) {
    ui.label("Camera resolution");
    egui::ComboBox::from_id_salt("camera-resolution")
        .selected_text(format!("{} × {}", options.width, options.height))
        .show_ui(ui, |ui| {
            for (width, height) in [
                (320, 180),
                (640, 360),
                (640, 480),
                (1280, 720),
                (1920, 1080),
            ] {
                if ui
                    .selectable_label(
                        (options.width, options.height) == (width, height),
                        format!("{width} × {height}"),
                    )
                    .clicked()
                {
                    options.width = width;
                    options.height = height;
                }
            }
        });
    ui.end_row();
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    struct Fixture {
        state: Rc<RefCell<CameraState>>,
        profiles: Rc<RefCell<Vec<CameraProfile>>>,
    }
    impl CameraBackend for Fixture {
        fn close(self: Box<Self>) -> blent_config::retirement::Fence {
            self.stop();
            blent_config::retirement::Fence::Complete
        }
        fn start(&self, options: CameraProfile) -> blent_config::camera::BackendResult {
            self.profiles.borrow_mut().push(options);
            *self.state.borrow_mut() = CameraState::Streaming;
            Ok(())
        }
        fn stop(&self) {
            *self.state.borrow_mut() = CameraState::Stopped;
        }
        fn state(&self) -> CameraState {
            self.state.borrow().clone()
        }
        fn preview(&self) -> Option<std::sync::Arc<blent_config::camera::CameraPreview>> {
            None
        }
    }

    #[test]
    fn t974_unsupported_invalid_and_ambiguous_camera_targets_never_create_backend() {
        use blent_config::camera::connection::Transports;
        let mut panel = Panel {
            transports: Transports::default(),
            create: || panic!("T974 unavailable target reached native factory"),
            ..Default::default()
        };
        assert_eq!(
            panel.start(CameraProfile::default()).unwrap_err(),
            "Selected camera transport is unavailable on this backend."
        );
        let valid = CameraProfile {
            usb_peer: Some("ab".repeat(32)),
            ..Default::default()
        };
        assert_eq!(
            panel.start(valid.clone()).unwrap_err(),
            "Selected camera transport is unavailable on this backend."
        );
        panel.transports = Transports {
            adb: true,
            usb: true,
        };
        for length in 0..=128 {
            if length == 64 {
                continue;
            }
            assert!(panel
                .start(CameraProfile {
                    usb_peer: Some("a".repeat(length)),
                    ..Default::default()
                })
                .is_err());
        }
        for value in [
            "A".repeat(64),
            "☃".repeat(64),
            format!("{}\n", "a".repeat(63)),
        ] {
            assert!(panel
                .start(CameraProfile {
                    usb_peer: Some(value),
                    ..Default::default()
                })
                .is_err());
        }
        assert!(panel
            .start(CameraProfile {
                serial: Some("other-tablet".into()),
                ..valid
            })
            .is_err());
        assert!(panel.backend.is_none());
    }

    #[test]
    fn t974_exact_usb_profile_reaches_only_camera_backend() {
        let profiles = Rc::new(RefCell::new(Vec::new()));
        let mut panel = Panel {
            backend: Some(Box::new(Fixture {
                state: Rc::new(RefCell::new(CameraState::Stopped)),
                profiles: profiles.clone(),
            })),
            ..Default::default()
        };
        let profile = CameraProfile {
            usb_peer: Some("ab".repeat(32)),
            lens: Lens::Rear,
            background: true,
            ..Default::default()
        };
        panel.start(profile.clone()).unwrap();
        assert_eq!(*profiles.borrow(), vec![profile]);
    }

    #[test]
    fn t974_unavailable_saved_target_remains_visible_and_rendering_never_rewrites_it() {
        use blent_config::camera::connection::Transports;
        let mut panel = Panel {
            transports: Transports::default(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        for usb in [false, true] {
            let original = CameraProfile {
                usb_peer: usb.then(|| "ab".repeat(32)),
                ..Default::default()
            };
            let mut profile = original.clone();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 1800.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        panel.show(ui, &mut profile);
                        egui::Grid::new("t974-target").show(ui, |ui| {
                            connection::show(ui, &mut profile, panel.transports)
                        });
                    });
                },
            );
            let mut text = Vec::new();
            for shape in output.shapes {
                crate::tests::collect_text_rects(&shape.shape, &mut text);
            }
            assert!(text
                .iter()
                .any(|(label, _)| label
                    == "Camera backend is not implemented on this operating system."));
            if usb {
                assert!(
                    text.iter()
                        .any(|(label, _)| label
                            == "Direct USB camera is unavailable on this backend.")
                );
                assert!(text
                    .iter()
                    .any(|(label, _)| label.starts_with("Experimental:")));
            }
            assert_eq!(profile, original);
            assert!(panel.backend.is_none());
        }
    }

    #[test]
    fn t543_portable_backend_contract_and_status_are_separate_from_preferences() {
        let state = Rc::new(RefCell::new(CameraState::Stopped));
        let profiles = Rc::new(RefCell::new(Vec::new()));
        let mut panel = Panel {
            backend: Some(Box::new(Fixture {
                state: state.clone(),
                profiles: profiles.clone(),
            })),
            error: None,
            ..Default::default()
        };
        assert_eq!(panel.state(), CameraState::Stopped);
        assert!(profiles.borrow().is_empty());
        assert!(panel
            .start(CameraProfile {
                fps: 0,
                ..Default::default()
            })
            .is_err());
        panel
            .start(CameraProfile {
                lens: Lens::Rear,
                background: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(profiles.borrow()[0].lens, Lens::Rear);
        assert_eq!(panel.state(), CameraState::Streaming);
        panel.backend.as_ref().unwrap().stop();
        assert_eq!(panel.state(), CameraState::Stopped);
        for state in [
            CameraState::Stopped,
            CameraState::Starting,
            CameraState::Waiting,
            CameraState::Streaming,
            CameraState::Stopping,
            CameraState::Failed("missing device".into()),
        ] {
            assert!(!state_label(&state).is_empty());
        }
        let mut failing = Panel {
            create: || Err("native runtime unavailable".into()),
            ..Default::default()
        };
        assert_eq!(
            failing.start(CameraProfile::default()).unwrap_err(),
            "native runtime unavailable"
        );
        drop(create_backend().unwrap()); // Creating the adapter starts no native camera work.
    }
}
