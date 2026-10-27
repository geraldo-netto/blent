use super::*;
use crate::tests::{click_settings_text, settings_test_app, sized_window_frame, window_test_frame};
use blent_config::{
    audio::{AudioController, AudioOptions, AudioStatus},
    camera::{CameraBackend, CameraProfile, CameraState},
    display_control::{Assignment, Lease, Phase, Snapshot, Target},
};
use std::sync::{Arc, Mutex};

struct Camera(Arc<Mutex<Vec<CameraProfile>>>);
impl CameraBackend for Camera {
    fn start(&self, options: CameraProfile) -> blent_config::camera::BackendResult {
        self.0.lock().unwrap().push(options);
        Ok(())
    }
    fn stop(&self) {}
    fn state(&self) -> CameraState {
        CameraState::Stopped
    }
    fn close(self: Box<Self>) -> blent_config::retirement::Fence {
        blent_config::retirement::Fence::Complete
    }
    fn preview(&self) -> Option<Arc<blent_config::camera::CameraPreview>> {
        None
    }
}
struct Audio(Arc<Mutex<Vec<AudioOptions>>>);
impl AudioController for Audio {
    fn start(&self, options: AudioOptions) -> blent_config::camera::BackendResult {
        self.0.lock().unwrap().push(options);
        Ok(())
    }
    fn stop(&self) {}
    fn status(&self) -> AudioStatus {
        AudioStatus::default()
    }
    fn close(self: Box<Self>) -> blent_config::retirement::Fence {
        blent_config::retirement::Fence::Complete
    }
}

fn devices() -> Vec<Device> {
    ["first", "second"]
        .into_iter()
        .map(|serial| {
            Device::new(
                Identity::Adb(serial.into()),
                format!("Tablet {serial}"),
                Presence::Connected,
                true,
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn t1273_media_only_catalog_entries_are_never_display_assignments() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    let media = Device::new(
        Identity::Adb("media-only".into()),
        "Media only".into(),
        Presence::Available,
        false,
    )
    .unwrap();
    let assigned = devices();
    let mut catalog = vec![media.clone()];
    catalog.extend(assigned);
    crate::tests::settings_test_frame(&mut app, &context, vec![], |_, ui| {
        ui.vertical(|ui| crate::overview::assignments(ui, &catalog));
    });
    let labels = crate::tests::settings_test_frame(&mut app, &context, vec![], |_, ui| {
        ui.vertical(|ui| crate::overview::assignments(ui, &catalog));
    });
    assert!(labels.iter().any(|(text, _)| text == "Tablet first"));
    assert!(!labels.iter().any(|(text, _)| text == "Media only"));
    let labels = crate::tests::settings_test_frame(&mut app, &context, vec![], |_, ui| {
        ui.vertical(|ui| crate::overview::assignments(ui, std::slice::from_ref(&media)));
    });
    assert!(labels
        .iter()
        .any(|(text, _)| text == "No display assignments"));
}

fn display() -> Snapshot {
    Snapshot {
        owner: "a".repeat(64),
        assignments: vec![Assignment {
            target: Target::Adb("first".into()),
            lease: Lease {
                slot: 0,
                generation: 1,
            },
            phase: Phase::Assigned,
        }],
    }
}

#[test]
fn t1273_display_owner_stays_visible_without_adb_inventory() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let status = Status {
        display: Some(display()),
        ..Default::default()
    };
    let devices = app.workspace_devices(&status);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].identity(), &Identity::Adb("first".into()));
    assert!(devices[0].display_assigned());
    assert_eq!(devices[0].presence(), Presence::Unknown);
    let mut invalid = display();
    invalid.owner.clear();
    let mut catalog = Catalog::new(1).unwrap();
    assert!(add_display_devices(&mut catalog, &invalid).is_err());
    assert!(catalog.devices().is_empty());
    let mut full = Catalog::new(1).unwrap();
    full.insert(devices[0].clone()).unwrap();
    let mut other = display();
    other.assignments[0].target = Target::Adb("second".into());
    assert!(add_display_devices(&mut full, &other).is_err());
    assert_eq!(full.devices().len(), 1);
    let status = Status {
        display: Some(invalid),
        ..Default::default()
    };
    assert!(app.workspace_devices(&status).is_empty());
    assert!(app.message.contains("invalid display owner"));
    assert!(app.feedback_area == crate::FeedbackArea::Display);
}

#[test]
fn t1273_tablet_workspace_retains_drafts_and_stops_only_selected_resource() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let camera = Arc::new(Mutex::new(Vec::new()));
    let audio = Arc::new(Mutex::new(Vec::new()));
    app.camera.backend = Some(Box::new(Camera(camera.clone())));
    app.microphone.backend = Some(Box::new(Audio(audio.clone())));
    app.speakers.backend = Some(Box::new(Audio(audio.clone())));
    {
        let mut status = app.status.lock().unwrap();
        status.tablets = devices();
        status.display = Some(display());
    }
    let draft = app.cfg.clone();
    let context = egui::Context::default();
    click_settings_text(&mut app, &context, "Start camera", window_test_frame);
    click_settings_text(&mut app, &context, "Start microphone", window_test_frame);
    click_settings_text(&mut app, &context, "Start speakers", window_test_frame);
    assert_eq!(camera.lock().unwrap()[0].serial.as_deref(), Some("first"));
    assert!(audio
        .lock()
        .unwrap()
        .iter()
        .all(|options| options.serial.as_deref() == Some("first")));
    assert_eq!(audio.lock().unwrap().len(), 2);
    assert_eq!(app.workspace_devices(&Status::default()).len(), 1);
    app.selected = Some(Identity::Adb("second".into()));
    let status = app.status.lock().unwrap().clone();
    assert_eq!(
        app.camera.view(app.selected.as_ref().unwrap()),
        ResourceView::Elsewhere(Activity::Starting)
    );
    app.selected = Some(Identity::Adb("first".into()));
    click_settings_text(&mut app, &context, "Stop camera", window_test_frame);
    assert!(app.camera.backend.is_none());
    assert!(app.microphone.backend.is_some());
    assert!(app.speakers.backend.is_some());
    assert_eq!(app.cfg, draft);
    assert_eq!(app.status.lock().unwrap().display, status.display);
    assert!(!app.busy());
    app.resource_action(
        &status,
        &Identity::Adb("first".into()),
        Resource::Microphone,
        Action::Stop,
    );
    assert!(app.microphone.backend.is_none());
    assert!(app.speakers.backend.is_some());
    app.resource_action(
        &status,
        &Identity::Adb("first".into()),
        Resource::Speakers,
        Action::Stop,
    );
    assert!(app.speakers.backend.is_none());
}

#[test]
fn t1273_pointer_gesture_cannot_cross_tablet_selection() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let starts = Arc::new(Mutex::new(Vec::new()));
    app.camera.backend = Some(Box::new(Camera(starts.clone())));
    app.status.lock().unwrap().tablets = devices();
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    let labels = window_test_frame(&mut app, &context, vec![]);
    let point = labels
        .iter()
        .find(|(text, _)| text == "Start camera")
        .unwrap()
        .1
        .center();
    for pressed in [true, false] {
        window_test_frame(
            &mut app,
            &context,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        app.selected = Some(Identity::Adb("second".into()));
    }
    assert!(
        starts.lock().unwrap().is_empty(),
        "T1273 old pointer gesture started a different tablet"
    );
}

#[test]
fn t1273_resource_helpers_preserve_exact_usb_and_adb_choices_and_all_states() {
    assert_eq!(presence(Presence::Connected), "Connected");
    assert_eq!(presence(Presence::Available), "Available to connect");
    assert_eq!(presence(Presence::Unknown), "Connection unconfirmed");
    let peer = blent_config::connection::pairing::PeerId::parse("b".repeat(64)).unwrap();
    let targets = [
        Identity::Adb("first".into()),
        Identity::UsbPeer(peer.clone()),
    ];
    for selected in &targets {
        assert_eq!(
            identity_key(selected).1,
            selected.adb_serial().unwrap_or(peer.as_str())
        );
        let mut serial = Some("old".into());
        let mut usb = Some("old".into());
        set_target(selected, &mut serial, &mut usb);
        assert_eq!(
            crate::media_owner::identity(
                blent_config::connection::media::Target::parse(serial.as_deref(), usb.as_deref())
                    .unwrap()
            )
            .as_ref(),
            Some(selected)
        );
        for adb in [false, true] {
            for usb in [false, true] {
                assert_eq!(
                    supports(Transports { adb, usb }, selected),
                    if selected.adb_serial().is_some() {
                        adb
                    } else {
                        usb
                    }
                );
            }
        }
    }
    assert_ne!(
        identity_key(&Identity::Adb(peer.as_str().into())),
        identity_key(&targets[1])
    );
    for value in [
        Activity::Unknown,
        Activity::Off,
        Activity::Starting,
        Activity::Waiting,
        Activity::InUse,
        Activity::Paused,
        Activity::Stopping,
        Activity::Failed,
    ] {
        assert!(!label(ResourceView::Here(value)).is_empty());
        assert!(label(ResourceView::Elsewhere(value)).contains("another tablet"));
    }
    assert!(!label(ResourceView::Unavailable).is_empty());
    assert!(!label(ResourceView::NotStarted).is_empty());
}

#[test]
fn t1273_display_projection_is_per_target_and_requires_current_owner_evidence() {
    let first = Identity::Adb("first".into());
    let second = Identity::Adb("second".into());
    let mut status = Status::default();
    assert_eq!(display_view(&status, &first), ResourceView::Unavailable);
    status.daemon_running = true;
    assert_eq!(
        display_view(&status, &first),
        ResourceView::Here(Activity::Unknown)
    );
    for (phase, activity) in [
        (Phase::Starting, Activity::Starting),
        (Phase::Assigned, Activity::InUse),
        (Phase::Stopping, Activity::Stopping),
    ] {
        let mut snapshot = display();
        snapshot.assignments[0].phase = phase;
        status.display = Some(snapshot);
        assert_eq!(display_view(&status, &first), ResourceView::Here(activity));
        assert_eq!(display_view(&status, &second), ResourceView::NotStarted);
    }
}

#[test]
fn t1273_workspace_selection_and_configuration_preserve_unrelated_settings() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    let status = Status {
        tablets: devices(),
        display: Some(display()),
        daemon_running: true,
        ..Default::default()
    };
    let catalog = app.workspace_devices(&status);
    app.reconcile_selection(&catalog);
    let first = app.selected.clone().unwrap();
    let draft = app.cfg.clone();
    for (resource, tab) in [
        (Resource::Display, Tab::Display),
        (Resource::Camera, Tab::Camera),
        (Resource::Microphone, Tab::Audio),
        (Resource::Speakers, Tab::Audio),
    ] {
        app.resource_action(&status, &first, resource, Action::Configure);
        assert!(app.tab == tab);
        assert_eq!(app.cfg, draft);
    }
    app.selected = Some(Identity::Adb("lost-tablet".into()));
    let catalog = app.workspace_devices(&Status::default());
    assert_eq!(catalog[0].presence(), Presence::Unknown);
    assert_eq!(catalog[0].identity(), app.selected.as_ref().unwrap());
    assert_eq!(app.workspace_devices(&status).len(), 3);
}

#[test]
fn t1273_workspace_is_adaptive_without_save_controls_on_home() {
    for size in [
        egui::vec2(380.0, 360.0),
        egui::vec2(440.0, 760.0),
        egui::vec2(1040.0, 740.0),
    ] {
        let (_root, mut app) = settings_test_app(Tab::Overview);
        app.status.lock().unwrap().tablets = devices();
        let context = egui::Context::default();
        sized_window_frame(&mut app, &context, vec![], size);
        let labels = sized_window_frame(&mut app, &context, vec![], size);
        assert!(!labels
            .iter()
            .any(|(text, _)| text.starts_with("Save") || text == "Discard"));
        for (text, rect) in &labels {
            assert!(rect.right() <= size.x, "T1273 clipped {text}: {rect:?}");
        }
        assert!(labels.iter().any(|(text, _)| text == "Display & input"));
    }
}

#[test]
fn t1273_resource_controls_expose_named_keyboard_actions() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    app.status.lock().unwrap().tablets = devices();
    let context = egui::Context::default();
    context.enable_accesskit();
    let output = context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1040.0, 740.0),
            )),
            ..Default::default()
        },
        |context| app.show_window(context),
    );
    let tree = output.platform_output.accesskit_update.unwrap();
    for title in [
        "Tablet first",
        "Connect tablet",
        "Configure display & input",
        "Start camera",
        "Start microphone",
        "Start speakers",
    ] {
        let node = tree
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.label() == Some(title))
            .unwrap();
        assert!(
            node.supports_action(egui::accesskit::Action::Focus),
            "T1273 unreachable {title}"
        );
        assert!(
            node.supports_action(egui::accesskit::Action::Click),
            "T1273 inactive {title}"
        );
    }
}

#[test]
fn t1273_connect_and_refresh_are_explicit_passive_workspace_actions() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    app.status.lock().unwrap().tablets = devices();
    let (observed, probes) = std::sync::mpsc::sync_channel(2);
    app._status_worker = Some(crate::status_worker::StatusWorker::start(
        move |force| {
            let _ = observed.send(force);
        },
        std::time::Duration::from_secs(60),
    ));
    assert!(probes.recv().unwrap());
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    click_settings_text(&mut app, &context, "Refresh tablets", window_test_frame);
    assert!(probes.recv().unwrap());
    click_settings_text(&mut app, &context, "Connect tablet", window_test_frame);
    assert!(app.tab == Tab::Connections);
    assert!(app.camera.backend.is_none());
    assert!(app.microphone.backend.is_none());
    assert!(app.speakers.backend.is_none());
    drop(app._status_worker.take());
    assert!(probes
        .recv_timeout(std::time::Duration::from_secs(1))
        .is_err());
}

// T1289: resize the same context in both directions, including maximized 4K.
#[test]
fn t1289_resource_grid_reflows_without_unbounded_card_widths() {
    let (_root, mut app) = settings_test_app(Tab::Overview);
    app.status.lock().unwrap().tablets = devices();
    let context = egui::Context::default();
    for (width, columns) in [
        (1040.0, 2),
        (3840.0, 4),
        (1920.0, 4),
        (380.0, 1),
        (1040.0, 2),
    ] {
        let size = egui::vec2(width, 1100.0);
        for _ in 0..3 {
            sized_window_frame(&mut app, &context, vec![], size);
        }
        let labels = sized_window_frame(&mut app, &context, vec![], size);
        let cards: Vec<_> = ["Display & input", "Camera", "Microphone", "Speakers"]
            .iter()
            .map(|name| labels.iter().find(|(text, _)| text == name).unwrap().1)
            .collect();
        let first_row = cards
            .iter()
            .filter(|rect| (rect.top() - cards[0].top()).abs() < 1.0)
            .count();
        assert_eq!(first_row, columns, "T1289 width {width}: {cards:?}");
        for pair in cards[..columns].windows(2) {
            assert!(
                pair[1].left() - pair[0].left() <= 500.0,
                "T1289 oversized cards: {cards:?}"
            );
        }
        if width == 3840.0 {
            assert!(
                cards[0].left() > 900.0,
                "T1289 center bounded workspace: {cards:?}"
            );
        }
        assert!(labels.iter().all(|(_, rect)| rect.right() <= width));
    }
}
