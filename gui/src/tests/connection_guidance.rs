//! T1270: dependency and connection instructions match the available route.
use super::*;

fn rendered(app: &mut App, mut render: impl FnMut(&mut App, &mut egui::Ui)) -> String {
    let context = egui::Context::default();
    let output = context.run(egui::RawInput::default(), |context| {
        egui::CentralPanel::default().show(context, |ui| render(app, ui));
    });
    let mut labels = Vec::new();
    for shape in output.shapes {
        collect_text_rects(&shape.shape, &mut labels);
    }
    labels
        .into_iter()
        .map(|(text, _)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn t1270_missing_adb_is_not_a_direct_usb_setup_failure() {
    let (_root, mut app) = settings_test_app(Tab::Video);
    app.status.lock().unwrap().adb_ok = false;
    let text = rendered(&mut app, |app, ui| {
        let status = app.status.lock().unwrap().clone();
        app.show_setup(ui, &status);
    });
    assert!(!text.contains("Setup needed"), "T1270: {text}");
    assert!(text.contains("ADB is optional"), "T1270: {text}");
    assert!(text.contains("camera or audio"), "T1270: {text}");
    assert!(!app.busy());
}

#[test]
fn t1270_disconnected_help_offers_direct_usb_without_debugging() {
    let (_root, mut app) = settings_test_app(Tab::Video);
    for running in [false, true] {
        app.status.lock().unwrap().daemon_running = running;
        let text = rendered(&mut app, |app, ui| {
            let status = app.status.lock().unwrap().clone();
            app.show_status(ui, &status);
        });
        assert!(text.contains("without USB debugging"), "T1270: {text}");
        assert!(
            !text.contains("enable USB debugging and allow"),
            "T1270: {text}"
        );
        assert!(!app.busy());
    }
}

#[test]
fn t1270_optional_adb_does_not_hide_display_prerequisites() {
    let (_root, mut app) = settings_test_app(Tab::Video);
    {
        let mut status = app.status.lock().unwrap();
        status.adb_ok = false;
        status.ffmpeg_ok = false;
        status.evdi_count = 0;
    }
    let text = rendered(&mut app, |app, ui| {
        let status = app.status.lock().unwrap().clone();
        app.show_setup(ui, &status);
    });
    assert!(text.contains("Setup needed"), "T1270: {text}");
    assert!(text.contains("ffmpeg"), "T1270: {text}");
    assert!(text.contains("Set up display and input"), "T1270: {text}");
    assert!(text.contains("ADB is optional"), "T1270: {text}");
}

#[test]
fn t1270_connection_help_matches_each_capability_combination() {
    let (_root, mut app) = settings_test_app(Tab::Video);
    for direct_usb in [false, true] {
        for adb in [false, true] {
            let text = rendered(&mut app, |_, ui| {
                crate::connection_guidance::show(ui, direct_usb, adb);
            });
            assert_eq!(
                text.contains("without USB debugging"),
                direct_usb,
                "T1270: {text}"
            );
            assert_eq!(text.contains("For an ADB connection"), adb, "T1270: {text}");
            assert_eq!(
                text.contains("No supported connection route"),
                !direct_usb && !adb,
                "T1270: {text}"
            );
        }
    }
}
