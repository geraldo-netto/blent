//! T1780: readable enabled selections and validation messages on light surfaces.
use super::*;

fn luminance(color: egui::Color32) -> f64 {
    [color.r(), color.g(), color.b()]
        .into_iter()
        .zip([0.2126, 0.7152, 0.0722])
        .map(|(value, weight)| {
            let channel = f64::from(value) / 255.0;
            weight
                * if channel <= 0.04045 {
                    channel / 12.92
                } else {
                    ((channel + 0.055) / 1.055).powf(2.4)
                }
        })
        .sum()
}

fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn t1780_selected_text_meets_normal_text_contrast() {
    let context = egui::Context::default();
    workspace_style::configure(&context);
    let style = context.style();
    let foreground = style
        .visuals
        .override_text_color
        .unwrap_or(style.visuals.selection.stroke.color);
    assert!(
        contrast(foreground, style.visuals.selection.bg_fill) >= 4.5,
        "T1780 selected text"
    );
    assert!(contrast(style.visuals.text_color(), style.visuals.panel_fill) >= 4.5);
}

#[test]
fn t1780_validation_text_meets_normal_text_contrast() {
    let (_root, mut app) = settings_test_app(Tab::Video);
    let context = egui::Context::default();
    workspace_style::configure(&context);
    app.cfg.fps = 0;
    for _ in 0..2 {
        let output = context.run(Default::default(), |context| {
            egui::CentralPanel::default()
                .show(context, |ui| assert!(!app.show_validation_error(ui)));
        });
        for shape in output.shapes {
            if let egui::Shape::Text(text) = shape.shape {
                for section in &text.galley.job.sections {
                    assert!(
                        contrast(section.format.color, context.style().visuals.panel_fill) >= 4.5,
                        "T1780 validation contrast"
                    );
                }
            }
        }
    }
    app.cfg.fps = 60;
    let _ = context.run(Default::default(), |context| {
        egui::CentralPanel::default().show(context, |ui| assert!(app.show_validation_error(ui)));
    });
}

#[test]
fn t1788_stopped_camera_keeps_image_controls_in_default_window() {
    let (_root, mut app) = settings_test_app(Tab::Camera);
    let context = egui::Context::default();
    let size = egui::vec2(1040.0, 740.0);
    sized_window_frame(&mut app, &context, vec![], size);
    let labels = sized_window_frame(&mut app, &context, vec![], size);
    let bottom = labels
        .iter()
        .find(|(text, _)| text == "Save & restart all displays")
        .unwrap()
        .1
        .top();
    for required in [
        "Start camera",
        "Lens",
        "Camera resolution",
        "Shadow correction",
        "Shadow strength",
    ] {
        let rect = labels.iter().find(|(text, _)| text == required).unwrap().1;
        assert!(
            rect.bottom() < bottom,
            "T1788 {required} below visible controls: {rect:?}"
        );
    }
    assert!(app.camera.backend.is_none());
}

#[test]
fn t1788_sensor_controls_are_available_without_transport_tuning() {
    let (_root, mut app) = settings_test_app(Tab::Camera);
    let context = egui::Context::default();
    click_settings_text(
        &mut app,
        &context,
        "Tablet image adjustments",
        window_test_frame,
    );
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(labels
        .iter()
        .any(|(label, _)| label == "Start the camera to discover supported controls."));
    assert!(app.camera.backend.is_none());
}

#[test]
fn t1792_general_prioritizes_preferences_and_compact_service_actions() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    let size = egui::vec2(1040.0, 740.0);
    sized_window_frame(&mut app, &context, vec![], size);
    let labels = sized_window_frame(&mut app, &context, vec![], size);
    let login = labels
        .iter()
        .find(|(text, _)| text == "Start Blent with the desktop")
        .unwrap()
        .1;
    let stop = labels
        .iter()
        .find(|(text, _)| text == "Stop display service (all tablets)")
        .unwrap()
        .1;
    assert!(
        login.top() < stop.top(),
        "T1792 preferences precede service controls"
    );
    assert!(login.bottom() < 600.0);
    assert!(!labels.iter().any(|(text, _)| text == "CPU scheduling"));
}

#[test]
fn t1792_advanced_app_settings_retain_scheduling_controls() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    fn tall(
        app: &mut App,
        context: &egui::Context,
        events: Vec<egui::Event>,
    ) -> Vec<(String, egui::Rect)> {
        sized_window_frame(app, context, events, egui::vec2(1040.0, 1400.0))
    }
    click_settings_text(&mut app, &context, "Advanced app settings", tall);
    let labels = tall(&mut app, &context, vec![]);
    assert!(labels.iter().any(|(text, _)| text == "CPU scheduling"));
}

#[test]
fn t1782_hidden_changes_remain_visible_and_name_restart_scope() {
    let (_root, mut app) = settings_test_app(Tab::About);
    let context = egui::Context::default();
    app.cfg.camera.options.mirror = !app.saved_cfg.camera.options.mirror;
    window_test_frame(&mut app, &context, vec![]);
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(
        labels
            .iter()
            .any(|(text, _)| text == "2 unsaved fields · all settings"),
        "T1782 missing global draft scope"
    );
    assert!(labels
        .iter()
        .any(|(text, _)| text == "Saving restarts display sharing for every tablet."));
    click_settings_text(
        &mut app,
        &context,
        "Review changed fields",
        window_test_frame,
    );
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(labels.iter().any(|(text, _)| text == "Cameras · mirror"));
    assert!(labels.iter().any(|(text, _)| text == "Video · width"));
    click_settings_text(&mut app, &context, "Video · width", window_test_frame);
    assert!(app.tab == Tab::Video);
    assert!(app.cfg != app.saved_cfg);
}

#[test]
fn t1782_invalid_hidden_field_keeps_save_disabled_and_reachable() {
    let (_root, mut app) = settings_test_app(Tab::Camera);
    app.cfg = app.saved_cfg.clone();
    app.cfg.fps = 0;
    let context = egui::Context::default();
    click_settings_text(
        &mut app,
        &context,
        "Review changed fields",
        window_test_frame,
    );
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(labels.iter().any(|(text, _)| text == "Video · fps"));
    click_settings_text(
        &mut app,
        &context,
        "Save & restart all displays",
        window_test_frame,
    );
    assert!(app.save.is_none());
    click_settings_text(
        &mut app,
        &context,
        "Review changed fields",
        window_test_frame,
    );
    click_settings_text(&mut app, &context, "Video · fps", window_test_frame);
    assert!(app.tab == Tab::Video);
}

#[test]
fn t1782_failed_save_preserves_named_pending_changes() {
    let (root, mut app) = settings_test_app(Tab::Camera);
    let path = root.path().join("broken-config.toml");
    std::fs::write(&path, "invalid = [").unwrap();
    app.store = ConfigStore::new(path);
    app.apply(false);
    wait_for_work(&mut app);
    let context = egui::Context::default();
    window_test_frame(&mut app, &context, vec![]);
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(labels
        .iter()
        .any(|(text, _)| text == "1 unsaved fields · all settings"));
    assert!(labels
        .iter()
        .any(|(text, _)| text.starts_with("Save failed:")));
    assert!(app.cfg != app.saved_cfg);
}
