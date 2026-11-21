//! Window closure preserves manually admitted media; Stop remains authoritative.
use eframe::egui;

impl crate::App {
    pub(super) fn handle_close(&mut self, context: &egui::Context) {
        if !context.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.media_is_active() {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            // Remains recoverable from the taskbar even without a daemon tray.
            context.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn media_is_active(&mut self) -> bool {
        self.camera.is_sharing() || self.microphone.is_sharing() || self.speakers.is_sharing()
    }
}
