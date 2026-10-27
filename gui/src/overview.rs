//! Stateless overview presentation. Assignment names are borrowed for this frame.
use eframe::egui;

pub(super) fn service_label(display: bool, running: bool) -> &'static str {
    match (display, running) {
        (true, true) => "Display service running",
        (true, false) => "Display service stopped",
        (false, true) => "Blent daemon running",
        (false, false) => "Blent daemon stopped",
    }
}

pub(super) fn assignments(ui: &mut egui::Ui, tablets: &[blent_config::workspace::Device]) {
    ui.add_space(10.0);
    ui.strong("Assigned tablets");
    if !tablets.iter().any(|device| device.display_assigned()) {
        ui.label("No display assignments");
    }
    for name in tablets.iter().filter(|device| device.display_assigned()) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new(name.name()).strong());
            ui.small("Display assigned");
        });
    }
    ui.small("Assignments do not establish that video is streaming. Pairing and connection details are in Connections.");
}
