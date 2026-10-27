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
