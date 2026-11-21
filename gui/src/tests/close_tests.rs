//! T1669: window closure cannot retire an active media owner.
use super::*;
use blent_config::camera::{CameraBackend, CameraProfile, CameraState};
use std::{cell::Cell, rc::Rc};

struct Camera(Rc<Cell<usize>>, CameraState);
impl Drop for Camera {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
impl CameraBackend for Camera {
    fn close(self: Box<Self>) -> blent_config::retirement::Fence {
        blent_config::retirement::Fence::Complete
    }
    fn start(&self, _: CameraProfile) -> blent_config::camera::BackendResult {
        unreachable!()
    }
    fn stop(&self) {
        unreachable!()
    }
    fn adjust(&self, _: &CameraProfile) -> blent_config::camera::AdjustmentResult {
        Ok(Default::default())
    }
    fn state(&self) -> CameraState {
        self.1.clone()
    }
    fn preview(&self) -> Option<std::sync::Arc<blent_config::camera::CameraPreview>> {
        None
    }
}

fn close_frame(app: &mut App, context: &egui::Context) -> egui::FullOutput {
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    context.run(input, |context| app.show_window(context))
}

#[test]
fn t1669_close_preserves_camera_and_reactivation_restores_original_state() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let drops = Rc::new(Cell::new(0));
    app.camera.backend = Some(Box::new(Camera(drops.clone(), CameraState::Streaming)));
    let context = egui::Context::default();
    let output = close_frame(&mut app, &context);
    let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
    assert!(commands.contains(&egui::ViewportCommand::CancelClose));
    assert!(commands.contains(&egui::ViewportCommand::Minimized(true)));
    assert_eq!(drops.get(), 0);
    let output = context.run(Default::default(), |context| activation::show(context));
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .contains(&egui::ViewportCommand::Minimized(false)));
    assert_eq!(
        app.camera.backend.as_ref().unwrap().state(),
        CameraState::Streaming
    );
    assert_eq!(drops.get(), 0);
    app.camera.stop();
    assert_eq!(drops.get(), 1);
    let output = close_frame(&mut app, &context);
    assert!(!output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .contains(&egui::ViewportCommand::CancelClose));
}

#[test]
fn t1669_close_retains_camera_intent_and_allows_terminal_states_to_exit() {
    for state in [
        CameraState::Starting,
        CameraState::Waiting,
        CameraState::Paused,
        CameraState::Streaming,
        CameraState::Stopping,
        CameraState::Stopped,
        CameraState::Failed("fixture".into()),
    ] {
        let (_root, mut app) = settings_test_app(Tab::Overview);
        let active = state.is_active();
        app.camera.backend = Some(Box::new(Camera(Rc::new(Cell::new(0)), state)));
        let output = close_frame(&mut app, &egui::Context::default());
        assert_eq!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::CancelClose),
            active
        );
    }
}

struct Audio(blent_config::audio::AudioState);
impl blent_config::audio::AudioController for Audio {
    fn close(self: Box<Self>) -> blent_config::retirement::Fence {
        blent_config::retirement::Fence::Complete
    }
    fn start(&self, _: blent_config::audio::AudioOptions) -> blent_config::camera::BackendResult {
        unreachable!()
    }
    fn stop(&self) {
        unreachable!()
    }
    fn status(&self) -> blent_config::audio::AudioStatus {
        blent_config::audio::AudioStatus {
            state: self.0,
            detail: "fixture".into(),
        }
    }
}

#[test]
fn t1669_close_preserves_each_audio_direction_independently() {
    use blent_config::audio::{AudioState, Direction};
    for direction in [Direction::Microphone, Direction::Speakers] {
        for state in [
            AudioState::Starting,
            AudioState::Streaming,
            AudioState::Paused,
            AudioState::Stopping,
            AudioState::Stopped,
            AudioState::Failed,
        ] {
            let (_root, mut app) = settings_test_app(Tab::Overview);
            let panel = match direction {
                Direction::Microphone => &mut app.microphone,
                Direction::Speakers => &mut app.speakers,
            };
            panel.backend = Some(Box::new(Audio(state)));
            let output = close_frame(&mut app, &egui::Context::default());
            assert_eq!(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .contains(&egui::ViewportCommand::CancelClose),
                state.is_active()
            );
        }
    }
}

#[test]
fn t1669_ordinary_frames_do_not_minimize_or_schedule_close_work() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    app.camera.backend = Some(Box::new(Camera(
        Rc::new(Cell::new(0)),
        CameraState::Streaming,
    )));
    let context = egui::Context::default();
    let output = context.run(Default::default(), |context| app.handle_close(context));
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .is_empty());
}

mod native;
