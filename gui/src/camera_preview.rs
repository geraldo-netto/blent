//! Preview is a thumbnail of the backend's exported pixels, never a second capture.
use blent_config::camera::{CameraPreview, Lens};
use eframe::egui;
use std::sync::Arc;

#[derive(Default)]
pub(super) struct Preview {
    frame: Option<Arc<CameraPreview>>,
    texture: Option<egui::TextureHandle>,
}

impl Preview {
    /// Prune independently of camera-tab rendering; the cache owns one texture
    /// and one thumbnail, and borrows no native capture resources.
    pub(super) fn maintain(&mut self, context: &egui::Context, live: bool) {
        let Some(frame) = &self.frame else {
            return;
        };
        if !live || !frame.fresh() {
            *self = Self::default();
            return;
        }
        context.request_repaint_after(std::time::Duration::from_millis(200));
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui, frame: Option<Arc<CameraPreview>>) {
        self.update(ui.ctx(), frame);
        ui.label("Preview");
        ui.vertical(|ui| {
            if let (Some(texture), Some(frame)) = (&self.texture, &self.frame) {
                ui.image((
                    texture.id(),
                    display_size(ui.available_width(), texture.size_vec2()),
                ));
                ui.label(match frame.lens() {
                    Lens::Front => "Front webcam output",
                    Lens::Rear => "Rear webcam output",
                });
            } else {
                ui.allocate_ui(egui::vec2(ui.available_width().min(640.0), 48.0), |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label("No live video · Start camera to preview")
                    });
                });
            }
        });
    }

    fn update(&mut self, context: &egui::Context, frame: Option<Arc<CameraPreview>>) {
        let Some(frame) = frame.filter(|frame| frame.fresh()) else {
            self.frame = None;
            self.texture = None;
            return;
        };
        if self
            .frame
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &frame))
        {
            return;
        }
        let image = egui::ColorImage::from_rgba_unmultiplied(frame.size(), frame.rgba());
        if let Some(texture) = &mut self.texture {
            texture.set(image, egui::TextureOptions::LINEAR);
        } else {
            self.texture =
                Some(context.load_texture("camera-preview", image, egui::TextureOptions::LINEAR));
        }
        self.frame = Some(frame);
    }
}

/// Frame-local geometry only: finite available width and positive image size
/// come from egui and the bounded CameraPreview contract. No retained state.
fn display_size(available: f32, image: egui::Vec2) -> egui::Vec2 {
    let width = available.clamp(0.0, 640.0);
    image * (width / image.x).min(480.0 / image.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1729_preview_fits_window_and_preserves_aspect() {
        for (available, source, expected) in [
            (960.0, [640.0, 480.0], [640.0, 480.0]),
            (320.0, [640.0, 480.0], [320.0, 240.0]),
            (960.0, [480.0, 640.0], [360.0, 480.0]),
            (960.0, [1920.0, 1080.0], [640.0, 360.0]),
            (0.0, [640.0, 480.0], [0.0, 0.0]),
            (-10.0, [640.0, 480.0], [0.0, 0.0]),
        ] {
            assert_eq!(display_size(available, source.into()), expected.into());
        }
    }

    #[test]
    fn t543_preview_uses_exported_lens_and_clears_texture_after_stop() {
        let context = egui::Context::default();
        let mut preview = Preview::default();
        for lens in [Lens::Front, Lens::Rear] {
            let frame = Arc::new(CameraPreview::new(lens, [2, 2], vec![127; 16]).unwrap());
            for _ in 0..2 {
                let _ = context.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| preview.show(ui, Some(frame.clone())));
                });
                assert_eq!(preview.frame.as_ref().unwrap().lens(), lens);
                assert_eq!(preview.texture.as_ref().unwrap().size(), [2, 2]);
            }
        }
        let _ = context.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| preview.show(ui, None));
        });
        assert!(preview.frame.is_none() && preview.texture.is_none());
    }
}
