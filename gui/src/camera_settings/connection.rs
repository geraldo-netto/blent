//! Portable camera target editing. Rendering never starts a connection or picks
//! an identity from display state. The backend retains native authority.
use blent_config::camera::{connection::Transports, CameraProfile};
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, options: &mut CameraProfile, transports: Transports) {
    ui.label("Connection");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                transports.adb,
                egui::RadioButton::new(options.usb_peer.is_none(), "ADB"),
            )
            .clicked()
        {
            options.usb_peer = None;
        }
        if ui
            .add_enabled(
                transports.usb,
                egui::RadioButton::new(options.usb_peer.is_some(), "Direct USB"),
            )
            .clicked()
        {
            if options.usb_peer.is_none() {
                options.usb_peer = Some(String::new());
            }
            options.serial = None;
        }
    });
    ui.end_row();
    if options.usb_peer.is_some() {
        usb(ui, options, transports.usb);
    } else {
        adb(ui, options, transports.adb);
    }
}

fn usb(ui: &mut egui::Ui, options: &mut CameraProfile, available: bool) {
    ui.label("Tablet fingerprint");
    let mut peer = options.usb_peer.clone().unwrap_or_default();
    if identifier(
        ui,
        &mut peer,
        "Full Tablet fingerprint from USB connection",
        available,
    ) {
        options.usb_peer = Some(peer.trim().into());
    }
    ui.end_row();
    ui.label("USB camera status");
    ui.vertical(|ui| {
        if !available { ui.label("Direct USB camera is unavailable on this backend."); }
        ui.label("Experimental: physical debugging-off camera acceptance is pending.");
        ui.label("Pair and select this tablet in USB connection controls. Start camera locally on the tablet; pairing does not grant camera access.");
    });
    ui.end_row();
}

fn adb(ui: &mut egui::Ui, options: &mut CameraProfile, available: bool) {
    ui.label("Tablet serial");
    let mut serial = options.serial.clone().unwrap_or_default();
    if identifier(
        ui,
        &mut serial,
        "Automatic (one connected tablet)",
        available,
    ) {
        options.serial = if serial.trim().is_empty() {
            None
        } else {
            Some(serial.trim().into())
        };
    }
    ui.end_row();
}

fn identifier(ui: &mut egui::Ui, value: &mut String, hint: &str, available: bool) -> bool {
    ui.add_enabled(
        available,
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(190.0_f32.min(ui.available_width())),
    )
    .changed()
}
