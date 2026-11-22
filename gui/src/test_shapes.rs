//! T1654: platform-neutral rendered-text inspection for GUI regressions.
use eframe::egui;

pub(crate) fn collect_text_rects(shape: &egui::Shape, text: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::Shape::Text(value) => text.push((
            value.galley.text().to_owned(),
            value.galley.rect.translate(value.pos.to_vec2()),
        )),
        egui::Shape::Vec(shapes) => {
            for shape in shapes {
                collect_text_rects(shape, text);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1793_text_bounds_include_alignment_offset() {
        let context = egui::Context::default();
        for alignment in [egui::Align::Min, egui::Align::Center, egui::Align::Max] {
            let _ = context.run(Default::default(), |context| {
                let mut job = egui::text::LayoutJob::simple("Aligned text".into(), egui::FontId::proportional(15.0), egui::Color32::BLACK, 300.0);
                job.halign = alignment;
                let galley = context.fonts(|fonts| fonts.layout_job(job));
                let anchor = egui::pos2(200.0, 20.0);
                let expected = galley.rect.translate(anchor.to_vec2());
                let shape = egui::Shape::Text(egui::epaint::TextShape::new(anchor, galley, egui::Color32::BLACK));
                let mut labels = Vec::new();
                collect_text_rects(&shape, &mut labels);
                assert_eq!(labels[0].1, expected, "T1793 aligned galley");
            });
        }
    }
}
