//! T1268: native overview, navigation, drafts and contextual feedback.
use super::*;

#[test]
fn t1268_initial_view_centres_connection_and_service_actions() {
    let (_root, mut app) = settings_test_app(Tab::default());
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(labels.iter().any(|(text, _)| text == "Tablet resources"));
    assert!(labels.iter().any(|(text, _)| text == "Open Connections"));
    assert!(
        labels.iter().any(|(text, _)| text == "Settings"),
        "labels: {labels:?}"
    );
    assert!(!labels.iter().any(|(text, _)| text == "Encoder"));
    assert!(!app.busy());
}

#[test]
fn t1268_assignments_have_individual_rows_even_with_commas_in_names() {
    use blent_config::tablets::{Identity, TabletSession};
    let (_root, mut app) = settings_test_app(Tab::default());
    let sessions = (0..2)
        .map(|instance| TabletSession {
            identity: Identity::Adb(format!("TABLET{instance}")),
            instance,
            video_port: 8890 + 2 * instance as u16,
            input_port: 8891 + 2 * instance as u16,
        })
        .collect();
    apply_tablet_sessions(
        &mut app.status.lock().unwrap(),
        "TABLET0 device model:First,_Tablet\nTABLET1 device model:Second\n",
        sessions,
    );
    let labels = sized_window_frame(
        &mut app,
        &egui::Context::default(),
        vec![],
        egui::vec2(1040.0, 740.0),
    );
    assert!(labels.iter().any(|(text, _)| text == "First, Tablet"));
    assert!(labels.iter().any(|(text, _)| text == "Second"));
    assert_eq!(
        app.status
            .lock()
            .unwrap()
            .tablets
            .iter()
            .filter(|device| device.display_assigned())
            .count(),
        2
    );
}

#[test]
fn t1268_navigation_retains_drafts_without_starting_media() {
    let (_root, mut app) = settings_test_app(Tab::default());
    let draft = app.cfg.clone();
    let saved = app.saved_cfg.clone();
    let context = egui::Context::default();
    click_settings_text(&mut app, &context, "Open Connections", window_test_frame);
    assert!(app.tab == Tab::Connections);
    click_settings_text(&mut app, &context, "Settings", window_test_frame);
    for title in [
        "Video",
        "Display & input",
        "Cameras",
        "Audio",
        "Settings",
        "Tablets",
    ] {
        click_settings_text(&mut app, &context, title, window_test_frame);
        assert_eq!(app.cfg, draft);
        assert_eq!(app.saved_cfg, saved);
        assert!(!app.busy());
        assert!(app.camera.backend.is_none());
        assert!(app.microphone.backend.is_none());
        assert!(app.speakers.backend.is_none());
    }
}

#[test]
fn t1268_display_operation_feedback_is_visible_by_its_control() {
    let (_root, mut app) = settings_test_app(Tab::default());
    let (sender, receiver) = std::sync::mpsc::channel();
    app.action = Some(receiver);
    app.feedback_area = FeedbackArea::Display;
    app.message = "Starting selected display service".into();
    let labels = settings_test_frame(&mut app, &egui::Context::default(), vec![], |app, ui| {
        ui.vertical(|ui| {
            let status = app.status.lock().unwrap().clone();
            app.show_status(ui, &status);
        });
    });
    assert!(labels
        .iter()
        .any(|(text, _)| text == "Starting selected display service"));
    sender
        .send("Display service failed: fixture error".into())
        .unwrap();
    app.poll_action();
    assert_eq!(app.message, "Display service failed: fixture error");
    assert!(!app.busy());
}

pub(super) fn key(key: egui::Key) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .collect()
}

#[test]
fn t1268_connection_navigation_supports_keyboard_activation() {
    let (_root, mut app) = settings_test_app(Tab::default());
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    let labels = window_test_frame(&mut app, &context, vec![]);
    let target = labels
        .iter()
        .find(|(text, _)| text == "Connections")
        .unwrap()
        .1;
    for _ in 0..20 {
        window_test_frame(&mut app, &context, key(egui::Key::Tab));
        let response = context
            .memory(|memory| memory.focused())
            .and_then(|id| context.read_response(id));
        if response.is_some_and(|response| response.rect.contains(target.center())) {
            window_test_frame(&mut app, &context, key(egui::Key::Enter));
            assert!(app.tab == Tab::Connections);
            assert!(!app.busy());
            return;
        }
    }
    panic!("T1268 Connections navigation has no reachable keyboard target");
}

#[test]
fn t1268_navigation_and_apply_stay_visible_when_content_scrolls() {
    for size in [
        egui::vec2(380.0, 560.0),
        egui::vec2(440.0, 760.0),
        egui::vec2(960.0, 360.0),
    ] {
        let (_root, mut app) = settings_test_app(Tab::Video);
        let context = egui::Context::default();
        sized_window_frame(&mut app, &context, vec![], size);
        let before = sized_window_frame(&mut app, &context, vec![], size);
        let navigation = before
            .iter()
            .find(|(text, _)| text == "Connections")
            .unwrap()
            .1;
        let events = vec![
            egui::Event::PointerMoved(egui::pos2(size.x / 2.0, size.y / 2.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -1400.0),
                modifiers: egui::Modifiers::NONE,
            },
        ];
        sized_window_frame(&mut app, &context, events, size);
        let after = sized_window_frame(&mut app, &context, vec![], size);
        assert!(after
            .iter()
            .any(|(text, rect)| text == "Connections" && *rect == navigation));
        assert!(after
            .iter()
            .any(|(text, rect)| text.starts_with("Save") && rect.bottom() <= size.y));
        for (text, rect) in after {
            assert!(rect.right() <= size.x, "T1268 clipped {text}: {rect:?}");
        }
    }
}

#[test]
fn t1268_service_labels_cover_running_and_unavailable_backends() {
    for (display, running, expected) in [
        (true, true, "Display service running"),
        (true, false, "Display service stopped"),
        (false, true, "Blent daemon running"),
        (false, false, "Blent daemon stopped"),
    ] {
        assert_eq!(crate::overview::service_label(display, running), expected);
    }
}

#[test]
fn t1268_bounded_assignment_names_wrap_without_horizontal_overflow() {
    for count in 0..=4 {
        for length in [0, 1, 32, 256] {
            let (_root, mut app) = settings_test_app(Tab::default());
            app.status.lock().unwrap().tablets = (0..count)
                .map(|index| {
                    blent_config::workspace::Device::new(
                        blent_config::tablets::Identity::Adb(format!("tablet-{index}")),
                        format!(
                            "Tablet {index}: {}",
                            "漢é,".repeat(length).chars().take(256).collect::<String>()
                        ),
                        blent_config::workspace::Presence::Unknown,
                        false,
                    )
                    .unwrap()
                })
                .collect();
            let context = egui::Context::default();
            let size = egui::vec2(380.0, 560.0);
            sized_window_frame(&mut app, &context, vec![], size);
            let labels = sized_window_frame(&mut app, &context, vec![], size);
            for (text, rect) in labels {
                assert!(rect.right() <= size.x, "T1268 clipped {text}: {rect:?}");
            }
            assert!(!app.busy());
        }
    }
}

#[test]
fn t1268_feedback_is_scoped_and_keeps_completed_errors_visible() {
    let (_root, mut app) = settings_test_app(Tab::default());
    app.feedback_area = FeedbackArea::Setup;
    app.message = "Setup failed: explicit fixture error".into();
    let labels = settings_test_frame(&mut app, &egui::Context::default(), vec![], |app, ui| {
        ui.vertical(|ui| {
            app.show_feedback(ui, FeedbackArea::Display);
            app.show_feedback(ui, FeedbackArea::Setup);
        });
    });
    assert_eq!(
        labels
            .iter()
            .filter(|(text, _)| text == &app.message)
            .count(),
        1
    );
    assert!(!app.busy());
}

#[test]
fn t1268_long_error_keeps_navigation_and_apply_visible_in_short_window() {
    let (_root, mut app) = settings_test_app(Tab::Video);
    app.message = "Native action failed with detailed fixture output. ".repeat(80);
    let context = egui::Context::default();
    let size = egui::vec2(380.0, 360.0);
    sized_window_frame(&mut app, &context, vec![], size);
    let labels = sized_window_frame(&mut app, &context, vec![], size);
    for title in ["Connections", "Save & restart all displays"] {
        let rect = labels.iter().find(|(text, _)| text == title).unwrap().1;
        assert!(
            rect.top() >= 0.0 && rect.bottom() <= size.y,
            "T1268 hidden {title}: {rect:?}"
        );
    }
}

#[test]
fn t1268_navigation_exposes_named_accessibility_actions() {
    let (_root, mut app) = settings_test_app(Tab::default());
    let context = egui::Context::default();
    context.enable_accesskit();
    let output = context.run(egui::RawInput::default(), |context| {
        app.show_window(context)
    });
    let tree = output.platform_output.accesskit_update.unwrap();
    for title in ["Connections", "Open Connections", "Settings"] {
        let node = tree
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.label() == Some(title))
            .unwrap();
        assert!(
            node.supports_action(egui::accesskit::Action::Focus),
            "T1268 unnamed focus action: {title}"
        );
        assert!(
            node.supports_action(egui::accesskit::Action::Click),
            "T1268 unnamed activation: {title}"
        );
    }
}
