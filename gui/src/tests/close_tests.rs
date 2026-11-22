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
    app.cfg = app.saved_cfg.clone();
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
    app.cfg = app.saved_cfg.clone();
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
    app.cfg = app.saved_cfg.clone();
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
    app.cfg = app.saved_cfg.clone();
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

#[test]
fn t1783_dirty_or_pending_preferences_cancel_native_close() {
    for pending in [false, true] {
        let (_root, mut app) = settings_test_app(Tab::Settings);
        let (sender, receiver) = std::sync::mpsc::channel();
        if pending {
            app.cfg = app.saved_cfg.clone();
            app.action = Some(action_feedback::Job::new(FeedbackArea::Settings, "Pending fixture", receiver));
        }
        let output = close_frame(&mut app, &egui::Context::default());
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.contains(&egui::ViewportCommand::CancelClose), "T1783 dirty/pending work was discarded");
        assert!(!commands.contains(&egui::ViewportCommand::Close));
        drop(sender);
        app.poll_action();
    }
}

#[test]
fn t1783_keep_editing_and_discard_are_explicit_choices() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    close_frame(&mut app, &context);
    click_settings_text(&mut app, &context, "Keep editing", window_test_frame);
    assert!(app.close_intent == window_lifecycle::CloseIntent::None);
    assert!(app.cfg != app.saved_cfg);
    close_frame(&mut app, &context);
    click_settings_text(&mut app, &context, "Discard and close", window_test_frame);
    assert!(app.close_intent == window_lifecycle::CloseIntent::Approved);
    assert_eq!(app.cfg, app.saved_cfg);
    assert!(app.save.is_none());
    let output = close_frame(&mut app, &context);
    assert!(!output.viewport_output[&egui::ViewportId::ROOT].commands.contains(&egui::ViewportCommand::CancelClose));
}

#[test]
fn t1783_save_on_close_waits_and_does_not_restart_displays() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    close_frame(&mut app, &context);
    click_settings_text(&mut app, &context, "Save preferences and close", window_test_frame);
    wait_for_work(&mut app);
    window_test_frame(&mut app, &context, vec![]);
    assert!(app.close_intent == window_lifecycle::CloseIntent::Approved);
    assert_eq!(app.message, "Settings saved");
    assert_eq!(app.cfg, app.store.load());
}

#[test]
fn t1783_failed_save_keeps_dialog_and_draft_for_retry() {
    let (root, mut app) = settings_test_app(Tab::Settings);
    let path = root.path().join("invalid.toml");
    std::fs::write(&path, "invalid = [").unwrap();
    app.store = ConfigStore::new(path);
    let context = egui::Context::default();
    close_frame(&mut app, &context);
    click_settings_text(&mut app, &context, "Save preferences and close", window_test_frame);
    wait_for_work(&mut app);
    window_test_frame(&mut app, &context, vec![]);
    assert!(app.close_intent == window_lifecycle::CloseIntent::Review);
    assert!(app.cfg != app.saved_cfg);
    assert!(app.message.starts_with("Save failed:"));
    click_settings_text(&mut app, &context, "Keep editing", window_test_frame);
    app.cfg.fps = 0;
    close_frame(&mut app, &context);
    click_settings_text(&mut app, &context, "Save preferences and close", window_test_frame);
    assert!(app.save.is_none());
}

#[test]
fn t1783_pending_save_cannot_be_discarded_or_duplicated_on_close() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    app.store.update(|_| Ok(())).unwrap();
    let lock = std::fs::File::open(_root.path().join("config.lock")).unwrap();
    lock.lock().unwrap();
    let context = egui::Context::default();
    close_frame(&mut app, &context);
    click_settings_text(&mut app, &context, "Save preferences and close", window_test_frame);
    assert!(app.busy());
    close_frame(&mut app, &context);
    click_settings_text(&mut app, &context, "Discard and close", window_test_frame);
    assert!(app.close_intent == window_lifecycle::CloseIntent::Saving);
    click_settings_text(&mut app, &context, "Keep editing", window_test_frame);
    assert!(app.busy());
    assert!(app.close_intent == window_lifecycle::CloseIntent::None);
    drop(lock);
    wait_for_work(&mut app);
    window_test_frame(&mut app, &context, vec![]);
    assert!(app.close_intent == window_lifecycle::CloseIntent::None);
    assert_eq!(app.cfg, app.store.load());
}
