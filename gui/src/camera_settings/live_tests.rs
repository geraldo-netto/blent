use super::*;
use std::{cell::Cell, rc::Rc};
struct Backend {
    mode: Rc<Cell<u8>>,
    updates: Rc<Cell<usize>>,
}
impl CameraBackend for Backend {
    fn close(self: Box<Self>) -> blent_config::retirement::Fence {
        blent_config::retirement::Fence::Complete
    }
    fn start(&self, _: CameraProfile) -> blent_config::camera::BackendResult {
        panic!("live edit restarted camera")
    }
    fn stop(&self) {
        panic!("live edit stopped camera")
    }
    fn state(&self) -> CameraState {
        CameraState::Streaming
    }
    fn preview(&self) -> Option<std::sync::Arc<blent_config::camera::CameraPreview>> {
        None
    }
    fn adjust(&self, profile: &CameraProfile) -> blent_config::camera::AdjustmentResult {
        profile.validate()?;
        self.updates.set(self.updates.get() + 1);
        Ok(blent_config::camera::adjustments::Status {
            capabilities: None,
            pending: self.mode.get() == 0,
            restart_required: self.mode.get() == 1,
            error: (self.mode.get() == 2).then(|| "Unsupported control".into()),
        })
    }
}
#[test]
fn t1467_live_render_updates_without_restart_and_surfaces_each_outcome() {
    let updates = Rc::new(Cell::new(0));
    let mode = Rc::new(Cell::new(0));
    let mut panel = Panel {
        backend: Some(Box::new(Backend {
            mode: mode.clone(),
            updates: updates.clone(),
        })),
        ..Default::default()
    };
    let context = egui::Context::default();
    let mut profile = CameraProfile::default();
    for value in 0..4 {
        mode.set(value);
        if value == 3 {
            profile.shadow_strength = 101;
        }
        let output = context.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.live_adjustments(ui, &profile));
        });
        assert!(!output.shapes.is_empty());
    }
    assert_eq!(updates.get(), 3);
    // Explicitly release fixture; no resource or native callback was admitted.
    panel.backend.take();
}

fn target_controller() -> (
    blent::camera_control::Controller,
    blent::control_workers::WorkerPool,
    std::sync::mpsc::Receiver<CameraProfile>,
) {
    let pool = blent::control_workers::WorkerPool::new(1).unwrap();
    let (sent, received) = std::sync::mpsc::channel();
    let controller = blent::camera_control::Controller::with_pool(
        pool.clone(),
        move |profile, mut stop, report| {
            report.update(CameraState::Streaming);
            sent.send(profile).unwrap();
            async move {
                blent::camera_control::cancelled(&mut stop).await;
                Ok(())
            }
        },
    )
    .unwrap();
    (controller, pool, received)
}

#[test]
fn t1548_overview_target_survives_live_adjustment_and_camera_tab_restart() {
    use crate::tests::{click_settings_text, settings_test_app, window_test_frame};
    use blent_config::tablets::Identity;
    use std::time::Duration;
    for selected in [
        Identity::Adb("T1548-tablet".into()),
        Identity::UsbPeer(
            blent_config::connection::pairing::PeerId::parse("ab".repeat(32)).unwrap(),
        ),
    ] {
        let (_root, mut app) = settings_test_app(crate::Tab::Camera);
        let (controller, pool, received) = target_controller();
        app.camera.backend = Some(Box::new(controller));
        let profile = app.cfg.camera.options.clone();
        app.camera.start_for(&selected, &profile);
        let admitted = received.recv_timeout(Duration::from_secs(2)).unwrap();
        app.cfg.camera.options.mirror = !profile.mirror;
        let context = egui::Context::default();
        let text = window_test_frame(&mut app, &context, Vec::new());
        let adjusted = text
            .iter()
            .any(|(label, _)| label == "Applying camera adjustments…");
        click_settings_text(&mut app, &context, "Restart camera", window_test_frame);
        let restarted = received.recv_timeout(Duration::from_secs(2));
        app.camera.stop();
        while pool.active() != 0 {
            std::thread::yield_now();
        }
        assert!(
            adjusted,
            "T1548 admitted target was lost during live adjustment: {text:?}"
        );
        let restarted = restarted.expect("T1548 camera-tab restart was rejected");
        assert_eq!(restarted.target().unwrap(), admitted.target().unwrap());
        assert_eq!(restarted.mirror, !profile.mirror);
        assert_eq!(
            app.cfg.camera.options.target().unwrap(),
            profile.target().unwrap()
        );
    }
}
