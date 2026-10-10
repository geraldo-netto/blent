//! Portable USB pairing widgets. Immutable views authorize commands; an independent
//! client owns native work and real retirement across GUI refresh/replacement.
use blent_config::connection::{
    control::{Command, State},
    control_client::{Client, Phase, Status},
    control_view::View,
    pairing::Ticket,
};
use eframe::egui;
use std::{collections::BTreeMap, time::Duration};
mod backend;

struct Choice {
    ticket: Ticket,
    remember: bool,
}
pub(super) struct Panel {
    source: Option<Box<dyn backend::Source>>,
    create: fn() -> Result<Box<dyn backend::Source>, String>,
    client: Option<Box<dyn Client>>,
    pending: Option<u64>,
    error: Option<String>,
    choices: BTreeMap<u64, Choice>,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            source: None,
            create: backend::create,
            client: None,
            pending: None,
            error: None,
            choices: BTreeMap::new(),
        }
    }
}
impl Panel {
    pub(super) fn show(&mut self, ui: &mut egui::Ui) {
        let status = self.refresh();
        ui.heading("USB connection");
        ui.label("Pair Blent with a tablet over USB without USB debugging.");
        self.controls(ui);
        if let Some(status) = &status {
            self.summary(ui, status);
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }
        if self.pending.is_some() {
            ui.label("Waiting for command confirmation…");
        }
        if let Some(view) = status.and_then(|status| status.view) {
            self.devices(ui, &view);
            self.links(ui, &view);
            self.saved(ui, &view);
        }
        if self.client.as_ref().is_some_and(|client| !client.retired()) {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }
    fn summary(&self, ui: &mut egui::Ui, status: &Status) {
        ui.label(phase_label(status.phase));
        if matches!(status.phase, Phase::Failed | Phase::Closed)
            && self.client.as_ref().is_some_and(|client| !client.retired())
        {
            ui.label("Waiting for native cleanup; retry unavailable.");
        }
    }
    fn refresh(&mut self) -> Option<Status> {
        let status = self.client.as_ref()?.status();
        if let Some(id) = self.pending {
            self.complete(&status, id);
        }
        if self.error.is_none() {
            self.error = status.message.clone();
        }
        self.choices.retain(|connection, choice| {
            status
                .view
                .as_ref()
                .and_then(|view| view.snapshot().links().get(connection))
                .is_some_and(|link| {
                    link.state == State::Pending && link.ticket.as_ref() == Some(&choice.ticket)
                })
        });
        Some(status)
    }
    fn complete(&mut self, status: &Status, id: u64) {
        if let Some(outcome) = status.outcome.as_ref().filter(|outcome| outcome.id == id) {
            self.pending = None;
            self.error = outcome.error.clone();
        } else if matches!(status.phase, Phase::Closing | Phase::Closed | Phase::Failed) {
            self.pending = None;
            self.error = Some(format!("Command result is unconfirmed. Reconnect and inspect current state before trying again. {}", status.message.as_deref().unwrap_or_default()));
        }
    }
    fn start(&mut self) -> Result<(), String> {
        if self.client.as_ref().is_some_and(|client| !client.retired()) {
            return Err("Previous connection controls are still stopping.".into());
        }
        if self.source.is_none() {
            self.source = Some((self.create)()?);
        }
        let client = self.source.as_mut().unwrap().start()?;
        self.client = Some(client);
        self.pending = None;
        self.choices.clear();
        Ok(())
    }
    fn send(&mut self, view: &View, command: Command) {
        if self.pending.is_some() {
            return;
        }
        let result = view
            .prepare(command)
            .map_err(|error| error.to_string())
            .and_then(|action| {
                self.client
                    .as_ref()
                    .ok_or("Connection controls are closed".to_string())?
                    .submit(action)
                    .map_err(|error| format!("Command not queued: {error:?}"))
            });
        match result {
            Ok(id) => {
                self.pending = Some(id);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        if !cfg!(target_os = "linux") {
            ui.label("ADB-free USB connections are not supported on this operating system.");
        }
        let available = self.client.as_ref().is_none_or(|client| client.retired());
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    available && cfg!(target_os = "linux"),
                    egui::Button::new("Find USB devices"),
                )
                .clicked()
            {
                self.error = self.start().err();
            }
            if ui
                .add_enabled(!available, egui::Button::new("Stop discovering"))
                .clicked()
            {
                if let Some(client) = &self.client {
                    client.close();
                }
            }
        });
        ui.small("Stopping discovery or closing this window leaves connected tablets running. Use Disconnect to stop a connection.");
    }
    fn devices(&mut self, ui: &mut egui::Ui, view: &View) {
        ui.separator();
        ui.strong("USB devices");
        if view.snapshot().devices().is_empty() {
            ui.label("No USB devices found. Connect your tablet and open Blent.");
        }
        for (target, device) in view.snapshot().devices() {
            ui.push_id(("device", target), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "Device {target}: {:04x}:{:04x}",
                        device.vendor, device.product
                    ));
                    if ui
                        .add_enabled(self.pending.is_none(), egui::Button::new("Connect"))
                        .clicked()
                    {
                        self.send(
                            view,
                            Command::Connect {
                                revision: view.snapshot().revision(),
                                target: *target,
                            },
                        );
                    }
                });
            });
        }
    }
    fn links(&mut self, ui: &mut egui::Ui, view: &View) {
        for (connection, link) in view.snapshot().links() {
            ui.push_id(("link", connection), |ui| {
                ui.separator();
                ui.label(format!(
                    "Connection {connection}: {}",
                    link_label(link.state)
                ));
                if let Some(ticket) = &link.ticket {
                    fingerprint(ui, "Computer", view.snapshot().computer().as_str());
                    fingerprint(ui, "Tablet", ticket.peer.as_str());
                    ui.label(if link.trusted {
                        "Tablet remembered"
                    } else {
                        "Tablet not remembered"
                    });
                    if link.state == State::Pending {
                        self.pair(ui, view, *connection, ticket);
                    }
                }
                if ui
                    .add_enabled(
                        self.pending.is_none()
                            && !matches!(link.state, State::Closing | State::Quarantined),
                        egui::Button::new("Disconnect"),
                    )
                    .clicked()
                {
                    self.send(
                        view,
                        Command::Disconnect {
                            connection: *connection,
                            ticket: link.ticket.clone(),
                        },
                    );
                }
            });
        }
    }
    fn pair(&mut self, ui: &mut egui::Ui, view: &View, connection: u64, ticket: &Ticket) {
        let choice = self.choices.entry(connection).or_insert_with(|| Choice {
            ticket: ticket.clone(),
            remember: false,
        });
        ui.label("Compare each fingerprint with the same label on the tablet. Approve only when both match.");
        ui.add_enabled(
            self.pending.is_none(),
            egui::Checkbox::new(&mut choice.remember, "Remember this tablet"),
        );
        let remember = choice.remember;
        ui.horizontal_wrapped(|ui| {
            for (label, allow) in [("Approve tablet", true), ("Deny", false)] {
                if ui
                    .add_enabled(self.pending.is_none(), egui::Button::new(label))
                    .clicked()
                {
                    self.send(
                        view,
                        Command::Decide {
                            connection,
                            ticket: ticket.clone(),
                            allow,
                            remember: allow && remember,
                        },
                    );
                }
            }
        });
    }
    fn saved(&mut self, ui: &mut egui::Ui, view: &View) {
        ui.separator();
        ui.strong("Remembered tablets");
        if view.snapshot().saved().is_empty() {
            ui.label("No tablets remembered.");
        }
        for peer in view.snapshot().saved().values() {
            ui.push_id(("saved", peer.as_str()), |ui| {
                fingerprint(ui, "Tablet", peer.as_str());
                if ui
                    .add_enabled(self.pending.is_none(), egui::Button::new("Forget tablet"))
                    .clicked()
                {
                    self.send(view, Command::Revoke { peer: peer.clone() });
                }
            });
        }
    }
}
fn fingerprint(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(format!("{label} fingerprint (SHA-256)"));
    // Fixed-sized digest is displayed in full and remains selectable/copyable.
    ui.add(
        egui::Label::new(egui::RichText::new(value).monospace())
            .wrap()
            .selectable(true),
    );
}
fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Connecting => "Connecting to USB service…",
        Phase::Ready => "USB status ready",
        Phase::Busy => "Refreshing USB status…",
        Phase::Closing => "Stopping discovery; waiting for cleanup…",
        Phase::Closed => "Discovery stopped",
        Phase::Failed => "USB connection controls failed",
    }
}
fn link_label(state: State) -> &'static str {
    match state {
        State::Discovering => "identifying tablet",
        State::Pending => "approval required",
        State::Approved => "verifying secure connection",
        State::Authenticated => "secure connection established",
        State::Closing => "disconnecting",
        State::Quarantined => "cleanup incomplete; replacement unavailable",
    }
}
#[cfg(test)]
mod tests;
