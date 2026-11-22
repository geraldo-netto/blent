//! T1780: readable enabled selections and validation messages on light surfaces.
use super::*;

fn luminance(color: egui::Color32) -> f64 {
    [color.r(), color.g(), color.b()].into_iter().zip([0.2126, 0.7152, 0.0722])
        .map(|(value, weight)| {
            let channel = f64::from(value) / 255.0;
            weight * if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
        }).sum()
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
    let foreground = style.visuals.override_text_color.unwrap_or(style.visuals.selection.stroke.color);
    assert!(contrast(foreground, style.visuals.selection.bg_fill) >= 4.5, "T1780 selected text");
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
            egui::CentralPanel::default().show(context, |ui| assert!(!app.show_validation_error(ui)));
        });
        for shape in output.shapes {
            if let egui::Shape::Text(text) = shape.shape {
                for section in &text.galley.job.sections {
                    assert!(contrast(section.format.color, context.style().visuals.panel_fill) >= 4.5, "T1780 validation contrast");
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
    let bottom = labels.iter().find(|(text, _)| text == "Save & restart all displays").unwrap().1.top();
    for required in ["Start camera", "Lens", "Camera resolution", "Shadow correction", "Shadow strength"] {
        let rect = labels.iter().find(|(text, _)| text == required).unwrap().1;
        assert!(rect.bottom() < bottom, "T1788 {required} below visible controls: {rect:?}");
    }
    assert!(app.camera.backend.is_none());
}

#[test]
fn t1788_sensor_controls_are_available_without_transport_tuning() {
    let (_root, mut app) = settings_test_app(Tab::Camera);
    let context = egui::Context::default();
    click_settings_text(&mut app, &context, "Tablet image adjustments", window_test_frame);
    let labels = window_test_frame(&mut app, &context, vec![]);
    assert!(labels.iter().any(|(label, _)| label == "Start the camera to discover supported controls."));
    assert!(app.camera.backend.is_none());
}

#[test]
fn t1792_general_prioritizes_preferences_and_compact_service_actions() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    let size = egui::vec2(1040.0, 740.0);
    sized_window_frame(&mut app, &context, vec![], size);
    let labels = sized_window_frame(&mut app, &context, vec![], size);
    let login = labels.iter().find(|(text, _)| text == "Start Blent with the desktop").unwrap().1;
    let stop = labels.iter().find(|(text, _)| text == "Stop display service (all tablets)").unwrap().1;
    assert!(login.top() < stop.top(), "T1792 preferences precede service controls");
    assert!(login.bottom() < 600.0);
    assert!(!labels.iter().any(|(text, _)| text == "CPU scheduling"));
}

#[test]
fn t1792_advanced_app_settings_retain_scheduling_controls() {
    let (_root, mut app) = settings_test_app(Tab::Settings);
    let context = egui::Context::default();
    fn tall(app: &mut App, context: &egui::Context, events: Vec<egui::Event>) -> Vec<(String, egui::Rect)> {
        sized_window_frame(app, context, events, egui::vec2(1040.0, 1400.0))
    }
    click_settings_text(&mut app, &context, "Advanced app settings", tall);
    let labels = tall(&mut app, &context, vec![]);
    assert!(labels.iter().any(|(text, _)| text == "CPU scheduling"));
}
