use blent_config::camera::tuning::Noise;
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, noise: &mut Noise, mask: u8) {
    ui.label("Noise reduction");
    egui::ComboBox::from_id_salt("camera-noise")
        .selected_text(label(*noise))
        .show_ui(ui, |ui| {
            for value in [Noise::Device, Noise::Off, Noise::Fast] {
                ui.add_enabled_ui(mask & (1 << value as u8) != 0, |ui| {
                    ui.selectable_value(noise, value, label(value));
                });
            }
        }).response.on_hover_text("Fast uses the camera's low-latency noise reduction. It may soften detail or behave like Off on some cameras.");
    ui.end_row();
}
fn label(noise: Noise) -> &'static str {
    match noise {
        Noise::Device => "Device default",
        Noise::Off => "Off",
        Noise::Fast => "Fast",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        context: &egui::Context,
        noise: &mut Noise,
        mask: u8,
        events: Vec<egui::Event>,
    ) -> Vec<(String, egui::Rect)> {
        let output = context.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::Grid::new("t1469").show(ui, |ui| show(ui, noise, mask));
                });
            },
        );
        let mut text = Vec::new();
        for shape in output.shapes {
            crate::tests::collect_text_rects(&shape.shape, &mut text);
        }
        text
    }
    fn click(context: &egui::Context, noise: &mut Noise, mask: u8, pos: egui::Pos2) {
        for pressed in [true, false] {
            frame(
                context,
                noise,
                mask,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
    }
    #[test]
    fn t1469_noise_choices_follow_device_capabilities() {
        for mask in [1, 3, 5, 7] {
            let context = egui::Context::default();
            let mut noise = Noise::default();
            let text = frame(&context, &mut noise, mask, vec![]);
            click(
                &context,
                &mut noise,
                mask,
                text.iter()
                    .find(|(name, _)| name == "Device default")
                    .unwrap()
                    .1
                    .center(),
            );
            let text = frame(&context, &mut noise, mask, vec![]);
            click(
                &context,
                &mut noise,
                mask,
                text.iter()
                    .find(|(name, _)| name == "Fast")
                    .unwrap()
                    .1
                    .center(),
            );
            assert_eq!(
                noise,
                if mask & 4 != 0 {
                    Noise::Fast
                } else {
                    Noise::Device
                }
            );
        }
        assert_eq!(label(Noise::Off), "Off");
    }
}
