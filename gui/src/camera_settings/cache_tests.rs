use super::*;
use blent_config::camera::CameraPreview;
use std::{cell::RefCell, rc::Rc, sync::Arc};

struct Backend {
    state: Rc<RefCell<CameraState>>,
    frame: Rc<RefCell<Option<Arc<CameraPreview>>>>,
}
impl CameraBackend for Backend {
    fn close(self: Box<Self>) -> blent_config::retirement::Fence {
        self.stop();
        blent_config::retirement::Fence::Complete
    }
    fn start(&self, _: CameraProfile) -> blent_config::camera::BackendResult {
        Ok(blent_config::media_start::Start::acknowledged())
    }
    fn stop(&self) {
        self.frame.borrow_mut().take();
    }
    fn state(&self) -> CameraState {
        self.state.borrow().clone()
    }
    fn preview(&self) -> Option<Arc<CameraPreview>> {
        self.frame.borrow().clone()
    }
}

fn populate(app: &mut crate::App, context: &egui::Context, frame: Arc<CameraPreview>) {
    let _ = context.run(Default::default(), |ctx| {
        egui::CentralPanel::default()
            .show(ctx, |ui| app.camera.preview.show(ui, Some(frame.clone())));
    });
}

#[test]
fn t1458_hidden_tab_releases_cache_on_backend_terminal_states() {
    for state in [
        CameraState::Stopped,
        CameraState::Failed("fixture".into()),
        CameraState::Waiting,
        CameraState::Paused,
    ] {
        let (_root, mut app) = crate::tests::settings_test_app(crate::Tab::About);
        let context = egui::Context::default();
        let frame = Arc::new(CameraPreview::new(Lens::Front, [2, 2], vec![127; 16]).unwrap());
        let weak = Arc::downgrade(&frame);
        populate(&mut app, &context, frame);
        let allocated = context.tex_manager().read().num_allocated();
        app.camera.backend = Some(Box::new(Backend {
            state: Rc::new(RefCell::new(state)),
            frame: Rc::new(RefCell::new(None)),
        }));
        crate::tests::window_test_frame(&mut app, &context, Vec::new());
        assert!(weak.upgrade().is_none(), "T1458 hidden tab retained pixels");
        assert_eq!(context.tex_manager().read().num_allocated(), allocated - 1);
    }
}

#[test]
fn t1458_hidden_tab_expires_cache_even_if_backend_keeps_frame() {
    let (_root, mut app) = crate::tests::settings_test_app(crate::Tab::About);
    let context = egui::Context::default();
    let frame = Arc::new(CameraPreview::new(Lens::Front, [2, 2], vec![127; 16]).unwrap());
    let weak = Arc::downgrade(&frame);
    let retained = Rc::new(RefCell::new(Some(frame.clone())));
    app.camera.backend = Some(Box::new(Backend {
        state: Rc::new(RefCell::new(CameraState::Streaming)),
        frame: retained.clone(),
    }));
    populate(&mut app, &context, frame);
    let allocated = context.tex_manager().read().num_allocated();
    crate::tests::window_test_frame(&mut app, &context, Vec::new());
    assert_eq!(context.tex_manager().read().num_allocated(), allocated);
    std::thread::sleep(blent_config::camera::FRESH_FRAME_AGE);
    crate::tests::window_test_frame(&mut app, &context, Vec::new());
    assert_eq!(
        context.tex_manager().read().num_allocated(),
        allocated - 1,
        "T1458 expired texture retained"
    );
    retained.borrow_mut().take();
    assert!(weak.upgrade().is_none());
}
