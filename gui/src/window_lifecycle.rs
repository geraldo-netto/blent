//! Closing retains the original transaction and media owners until settled.
//! This enum records only UI intent, never a duplicate draft or native lease.
use eframe::egui;

#[derive(Default, PartialEq, Eq)]
pub(super) enum CloseIntent {
    #[default]
    None,
    Review,
    Saving,
    Approved,
}

enum Choice {
    None,
    Save,
    Discard,
    KeepEditing,
}

impl crate::App {
    pub(super) fn handle_close(&mut self, context: &egui::Context) {
        if !context.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.media_is_active() {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.minimize_for_media(context);
            return;
        }
        if self.close_intent == CloseIntent::Approved {
            return;
        }
        if self.cfg != self.saved_cfg || self.busy() {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.close_intent != CloseIntent::Saving {
                self.close_intent = CloseIntent::Review;
            }
        }
    }

    fn media_is_active(&mut self) -> bool {
        self.camera.is_sharing() || self.microphone.is_sharing() || self.speakers.is_sharing()
    }

    fn minimize_for_media(&mut self, context: &egui::Context) {
        self.close_intent = CloseIntent::None;
        context.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    fn finish_window_close(&mut self, context: &egui::Context) {
        if self.media_is_active() {
            self.minimize_for_media(context);
        } else {
            self.close_intent = CloseIntent::Approved;
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub(super) fn show_close_dialog(&mut self, context: &egui::Context) {
        if self.close_intent == CloseIntent::Saving && !self.busy() {
            if self.cfg == self.saved_cfg {
                self.finish_window_close(context);
            } else {
                self.close_intent = CloseIntent::Review;
            }
        }
        if !matches!(self.close_intent, CloseIntent::Review | CloseIntent::Saving) {
            return;
        }
        let choice = egui::Modal::new(egui::Id::new("close-preferences"))
            .show(context, |ui| self.close_choices(ui))
            .inner;
        self.choose_close(context, choice);
    }

    fn choose_close(&mut self, context: &egui::Context, choice: Choice) {
        match choice {
            Choice::None => {}
            Choice::KeepEditing => self.close_intent = CloseIntent::None,
            Choice::Save => {
                self.apply(false);
                self.close_intent = CloseIntent::Saving;
            }
            Choice::Discard => {
                self.cfg = self.saved_cfg.clone();
                self.finish_window_close(context);
            }
        }
    }

    fn close_choices(&self, ui: &mut egui::Ui) -> Choice {
        ui.set_max_width(420.0);
        ui.heading("Close Blent?");
        ui.label("Save preferences without restarting displays, discard unsaved preferences, or keep editing.");
        ui.small("Display changes saved here take effect when display sharing next starts.");
        if self.busy() {
            ui.label("Waiting for the current action to finish.");
        }
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        let valid = self.show_validation_error(ui);
        close_buttons(ui, !self.busy(), valid && self.cfg != self.saved_cfg)
    }
}

fn close_buttons(ui: &mut egui::Ui, ready: bool, save_valid: bool) -> Choice {
    let mut choice = Choice::None;
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                ready && save_valid,
                egui::Button::new("Save preferences and close"),
            )
            .clicked()
        {
            choice = Choice::Save;
        }
        if ui
            .add_enabled(ready, egui::Button::new("Discard and close"))
            .clicked()
        {
            choice = Choice::Discard;
        }
        if ui.button("Keep editing").clicked() {
            choice = Choice::KeepEditing;
        }
    });
    choice
}
