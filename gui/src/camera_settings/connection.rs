//! Camera projection to the shared independent media target editor.
use crate::media_target_settings::{self, Description, Selection};
use blent_config::camera::{connection::Transports, CameraProfile};
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, options: &mut CameraProfile, transports: Transports) {
    media_target_settings::show(ui, Selection {serial: &mut options.serial, usb_peer: &mut options.usb_peer}, transports,
        Description {label:"USB camera status", unavailable:"Direct USB camera is unavailable on this backend.", qualification:"Experimental: physical debugging-off camera acceptance is pending.",
            consent:"Pair and select this tablet in USB connection controls. Start camera locally on the tablet; pairing does not grant camera access."});
}
