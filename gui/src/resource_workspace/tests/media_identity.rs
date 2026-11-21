//! T1650: card identity is authority; a saved request cannot change its resource.
use super::*;
use blent_config::{
    audio::{AudioController, AudioOptions, AudioStatus, Direction},
    media_start::Start,
    retirement::Fence,
    tablets::Identity,
    workspace::Resource,
};
use std::{cell::RefCell, rc::Rc};

struct Backend(Rc<RefCell<Vec<AudioOptions>>>);
impl AudioController for Backend {
    fn close(self: Box<Self>) -> Fence {
        Fence::Complete
    }
    fn start(&self, options: AudioOptions) -> blent_config::camera::BackendResult {
        self.0.borrow_mut().push(options);
        Ok(Start::acknowledged())
    }
    fn stop(&self) {}
    fn status(&self) -> AudioStatus {
        AudioStatus::default()
    }
}

fn install_backend(app: &mut App, resource: Resource) -> Rc<RefCell<Vec<AudioOptions>>> {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let panel = match resource {
        Resource::Microphone => &mut app.microphone,
        Resource::Speakers => &mut app.speakers,
        _ => panic!("invalid fixture resource"),
    };
    panel.backend = Some(Box::new(Backend(calls.clone())));
    calls
}

fn mismatch(app: &mut App, resource: Resource) {
    match resource {
        Resource::Microphone => app.cfg.audio.microphone.profile.direction = Direction::Speakers,
        Resource::Speakers => app.cfg.audio.speakers.profile.direction = Direction::Microphone,
        _ => panic!("invalid fixture resource"),
    }
}

#[test]
fn t1650_workspace_rejects_opposite_audio_direction_before_native_start() {
    for resource in [Resource::Microphone, Resource::Speakers] {
        let (_root, mut app) = settings_test_app(resource_tab(resource));
        let starts = install_backend(&mut app, resource);
        mismatch(&mut app, resource);
        let status = app.status.lock().unwrap().clone();
        let selected = Identity::Adb("t1650-tablet".into());
        app.start_resource(&status, &selected, resource);
        assert!(
            starts.borrow().is_empty(),
            "T1650: card started the opposite audio direction"
        );
        let (_, error) = app.resource_view(&status, &selected, resource);
        assert!(error.is_some(), "T1650: rejection reason missing");
    }
}
