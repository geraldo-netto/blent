//! Presentation of supplied route capabilities. Rendering owns no native work.
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, direct_usb: bool, adb: bool) {
    if direct_usb {
        ui.label("Pair in Connections without USB debugging. Open Blent on your tablet and select Find USB devices.");
    }
    if adb {
        ui.small("For an ADB connection, enable USB debugging and authorize this computer on your tablet.");
    }
    if !direct_usb && !adb {
        ui.label("No supported connection route is available. Check backend support and ADB installation.");
    }
}

/// Observed identities inform guidance only; no connection is created or changed.
pub(super) fn routes(
    ui: &mut egui::Ui,
    adb: bool,
    devices: &[blent_config::workspace::Device],
) -> bool {
    ui.heading("Choose a connection route");
    ui.label("Direct USB and USB debugging (ADB) are separate connection modes. Blent does not switch modes here.");
    ui.heading("USB debugging (ADB)");
    if adb {
        ui.label("ADB tablets appear automatically in Tablets. For USB, enable USB debugging and authorize this computer on the tablet.");
        ui.small("An ADB identity can also use an already configured network connection; it does not by itself identify the cable route.");
    } else {
        ui.label("ADB is unavailable. Install Android Platform Tools to use USB debugging, or use Direct USB below.");
    }
    observed_adb(ui, devices);
    let open = ui.button("Open Tablets").clicked();
    ui.separator();
    open
}

fn observed_adb(ui: &mut egui::Ui, devices: &[blent_config::workspace::Device]) {
    for device in devices {
        if matches!(device.identity(), blent_config::tablets::Identity::Adb(_)) {
            ui.label(format!("Observed ADB tablet: {}", device.name()));
        }
    }
}
