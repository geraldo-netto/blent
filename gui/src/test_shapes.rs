//! T1654: platform-neutral rendered-text inspection for GUI regressions.
use eframe::egui;

pub(crate) fn collect_text_rects(shape: &egui::Shape, text: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::Shape::Text(value) => text.push((
            value.galley.text().to_owned(),
            egui::Rect::from_min_size(value.pos, value.galley.size()),
        )),
        egui::Shape::Vec(shapes) => {
            for shape in shapes {
                collect_text_rects(shape, text);
            }
        }
        _ => {}
    }
}
