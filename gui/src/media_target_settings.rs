//! Shared target editing depends on independent media selection contracts.
//! Rendering never starts a connection or infers authority from display state.
use blent_config::connection::media::Transports;
use eframe::egui;

pub(super) struct Selection<'a> {
    pub(super) serial: &'a mut Option<String>,
    pub(super) usb_peer: &'a mut Option<String>,
}
pub(super) struct Description {
    pub(super) label: &'static str,
    pub(super) unavailable: &'static str,
    pub(super) qualification: &'static str,
    pub(super) consent: &'static str,
}
pub(super) fn show(
    ui: &mut egui::Ui,
    mut options: Selection<'_>,
    transports: Transports,
    description: Description,
) {
    ui.label("Connection");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                transports.adb,
                egui::RadioButton::new(options.usb_peer.is_none(), "ADB"),
            )
            .clicked()
        {
            *options.usb_peer = None;
        }
        if ui
            .add_enabled(
                transports.usb,
                egui::RadioButton::new(options.usb_peer.is_some(), "Direct USB"),
            )
            .clicked()
        {
            if options.usb_peer.is_none() {
                *options.usb_peer = Some(String::new());
            }
            *options.serial = None;
        }
    });
    ui.end_row();
    if options.usb_peer.is_some() {
        usb(ui, &mut options, transports.usb, description);
    } else {
        adb(ui, &mut options, transports.adb);
    }
}

fn usb(ui: &mut egui::Ui, options: &mut Selection<'_>, available: bool, description: Description) {
    ui.label("Tablet fingerprint");
    let mut peer = options.usb_peer.clone().unwrap_or_default();
    if identifier(ui, &mut peer, "Paste fingerprint", available) {
        *options.usb_peer = Some(peer.trim().into());
    }
    ui.end_row();
    ui.label(description.label);
    ui.vertical(|ui| {
        if !available {
            ui.label(description.unavailable);
        }
        ui.label(description.qualification);
        ui.label(description.consent);
    });
    ui.end_row();
}

fn adb(ui: &mut egui::Ui, options: &mut Selection<'_>, available: bool) {
    ui.label("Tablet serial");
    let mut serial = options.serial.clone().unwrap_or_default();
    if identifier(ui, &mut serial, "Auto (one tablet)", available) {
        *options.serial = if serial.trim().is_empty() {
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
    .on_hover_text("Use the full tablet fingerprint from Connections for direct USB. Leave the ADB serial empty to select one connected tablet automatically.")
    .changed()
}
