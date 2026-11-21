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
