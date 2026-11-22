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
