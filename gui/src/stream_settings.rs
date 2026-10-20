//! Stream presets reuse existing capture geometry and integer scaling contracts.
use blent_config::FileConfig;
use eframe::egui;

pub(crate) fn low_resolutions(ui: &mut egui::Ui, configuration: &mut FileConfig) {
    ui.horizontal_wrapped(|ui| {
        for (scale, label) in [(4, "180p (320 × 180)"), (2, "360p (640 × 360)")] {
            if ui.button(label).clicked() {
                configuration.auto_resolution = false;
                configuration.width = 1280;
                configuration.height = 720;
                configuration.stream_scale = scale;
            }
        }
    });
    ui.label(
        egui::RichText::new(
            "These presets use a 1280 × 720 desktop. Lower streamed detail; FPS and bitrate stay as selected. Restart display sharing to apply.",
        )
        .weak()
        .size(11.0),
    );
}
