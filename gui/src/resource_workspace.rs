//! Stateless frame presentation; owners and native effects stay in their panels.
use crate::{App, Status, Tab};
use blent_config::workspace::{Catalog, Device, Presence};
use blent_config::{
    connection::media::Transports,
    tablets::Identity,
    workspace::{Activity, Resource, ResourceView},
};
use eframe::egui;

// Four resources stay in complete rows; widths are egui logical points.
fn resource_columns(width: f32) -> usize {
    if width >= 1280.0 {
        4
    } else if width >= 620.0 {
        2
    } else {
        1
    }
}

impl App {
    pub(super) fn workspace_devices(&mut self, status: &Status) -> Vec<Device> {
        let result = self.collect_workspace_devices(status);
        match result {
            Ok(catalog) => catalog.into_devices(),
            Err(error) => {
                self.message = format!("Tablet list incomplete: {error}");
                self.feedback_area = crate::FeedbackArea::Display;
                status.tablets.iter().take(1024).cloned().collect()
            }
        }
    }

    fn collect_workspace_devices(&mut self, status: &Status) -> Result<Catalog, String> {
        let mut catalog = Catalog::new(1024).unwrap();
        for device in status
            .tablets
            .iter()
            .cloned()
            .chain(self.connection.tablets())
        {
            catalog.insert(device).map_err(|error| error.to_string())?;
        }
        if let Some(display) = &status.display {
            add_display_devices(&mut catalog, display)?;
        }
        for target in [
            self.camera.target(),
            self.microphone.target(),
            self.speakers.target(),
            self.selected.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if !catalog
                .devices()
                .iter()
                .any(|device| device.identity() == target)
            {
                let device = Device::new(target.clone(), target.label(), Presence::Unknown, false)
                    .map_err(|error| error.to_string())?;
                catalog.insert(device).map_err(|error| error.to_string())?;
            }
        }
        Ok(catalog)
    }

    pub(super) fn reconcile_selection(&mut self, devices: &[Device]) {
        if self.selected.is_none() {
            self.selected = devices.first().map(|device| device.identity().clone());
        }
    }

    pub(super) fn tablet_selector(&mut self, ui: &mut egui::Ui, devices: &[Device]) {
        if ui.ctx().screen_rect().width() < 860.0 {
            let selected = devices
                .iter()
                .find(|device| Some(device.identity()) == self.selected.as_ref())
                .map_or("Choose tablet", |device| device.name());
            egui::ComboBox::from_id_salt("selected-tablet")
                .selected_text(selected)
                .width((ui.available_width() - 40.0).max(0.0))
                .wrap_mode(egui::TextWrapMode::Truncate)
                .show_ui(ui, |ui| self.tablet_choices(ui, devices));
            if let Some(device) = devices
                .iter()
                .find(|device| Some(device.identity()) == self.selected.as_ref())
            {
                ui.small(presence(device.presence()));
            }
        } else {
            self.tablet_choices(ui, devices);
        }
        self.tablet_tools(ui);
    }

    fn tablet_tools(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Connect tablet").clicked() {
                self.tab = Tab::Connections;
            }
            if ui.button("Refresh tablets").clicked() {
                if let Some(worker) = &self._status_worker {
                    worker.refresh();
                }
            }
        });
    }

    fn tablet_choices(&mut self, ui: &mut egui::Ui, devices: &[Device]) {
        for device in devices {
            let selected = ui
                .push_id(identity_key(device.identity()), |ui| {
                    ui.selectable_label(
                        self.selected.as_ref() == Some(device.identity()),
                        device.name(),
                    )
                    .on_hover_text(device.identity().label())
                    .clicked()
                })
                .inner;
            if selected {
                self.selected = Some(device.identity().clone());
            }
            ui.small(presence(device.presence()));
        }
    }

    pub(super) fn show_resources(&mut self, ui: &mut egui::Ui, status: &Status) {
        if let Some(error) = &status.inventory_error {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Tablet discovery: {error}"),
            );
        }
        let Some(selected) = self.selected.clone() else {
            ui.heading("Connect your first tablet");
            ui.label("Use your tablet as a display, camera, microphone or speakers.");
            if ui.button("Open Connections").clicked() {
                self.tab = Tab::Connections;
            }
            crate::connection_guidance::show(ui, cfg!(target_os = "linux"), status.adb_ok);
            return;
        };
        ui.small(format!("Selected tablet: {}", selected.label()));
        ui.small("Each Start/Stop affects only this tablet and resource. Keep this window open for camera and audio.");
        self.show_feedback(ui, crate::FeedbackArea::Display);
        let resources = [
            Resource::Display,
            Resource::Camera,
            Resource::Microphone,
            Resource::Speakers,
        ];
        let columns = resource_columns(ui.available_width());
        for row in resources.chunks(columns) {
            ui.columns(columns, |columns| {
                for (column, resource) in columns.iter_mut().zip(row) {
                    self.resource_card(column, status, &selected, *resource);
                }
            });
            ui.add_space(12.0);
        }
        ui.small("Saving preferences never starts camera or audio. External media activity may be unconfirmed.");
    }

    fn resource_card(
        &mut self,
        ui: &mut egui::Ui,
        status: &Status,
        selected: &Identity,
        resource: Resource,
    ) {
        let (view, error) = self.resource_view(status, selected, resource);
        if resource == Resource::Display && !status.daemon_running {
            ui.small("Display service stopped. Start service in Settings; it discovers all eligible tablets.");
        }
        let enabled = resource != Resource::Display || (!self.busy() && status.display.is_some());
        let action = ui
            .push_id(identity_key(selected), |ui| {
                card(ui, resource, view, enabled, error.as_deref())
            })
            .inner;
        if let Some(action) = action {
            self.resource_action(status, selected, resource, action);
        }
        if view.can_stop() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn resource_view(
        &mut self,
        status: &Status,
        selected: &Identity,
        resource: Resource,
    ) -> (ResourceView, Option<String>) {
        use blent_config::audio::Direction;
        match resource {
            Resource::Display => (display_view(status, selected), status.display_error.clone()),
            Resource::Camera => (
                self.camera.view(selected),
                self.camera.error().map(str::to_owned),
            ),
            Resource::Microphone => (
                self.microphone.view(selected, Direction::Microphone),
                self.microphone.error().map(str::to_owned),
            ),
            Resource::Speakers => (
                self.speakers.view(selected, Direction::Speakers),
                self.speakers.error().map(str::to_owned),
            ),
        }
    }

    fn resource_action(
        &mut self,
        status: &Status,
        selected: &Identity,
        resource: Resource,
        action: Action,
    ) {
        match action {
            Action::Configure => self.tab = resource_tab(resource),
            Action::Start => self.start_resource(status, selected, resource),
            Action::Stop => self.stop_resource(status, selected, resource),
        }
    }

    fn start_resource(&mut self, status: &Status, selected: &Identity, resource: Resource) {
        match resource {
            Resource::Display => self.display_action(status, selected, true),
            Resource::Camera => self.camera.start_for(selected, &self.cfg.camera.options),
            Resource::Microphone => self
                .microphone
                .start_for(selected, &self.cfg.audio.microphone),
            Resource::Speakers => self.speakers.start_for(selected, &self.cfg.audio.speakers),
        }
    }

    fn stop_resource(&mut self, status: &Status, selected: &Identity, resource: Resource) {
        match resource {
            Resource::Display => self.display_action(status, selected, false),
            Resource::Camera => self.camera.stop(),
            Resource::Microphone => self.microphone.stop(),
            Resource::Speakers => self.speakers.stop(),
        }
    }

    pub(super) fn display_action(&mut self, status: &Status, selected: &Identity, enabled: bool) {
        let Some(snapshot) = &status.display else {
            return;
        };
        let target = blent_config::display_control::Target::from_identity(selected);
        let expected = snapshot
            .assignments
            .iter()
            .find(|assignment| assignment.target == target)
            .map(|assignment| assignment.lease);
        let command = blent_config::display_control::Command::Set {
            owner: snapshot.owner.clone(),
            target,
            enabled,
            expected: if enabled { None } else { expected },
        };
        let pid = status.daemon_pid;
        self.run_action(crate::FeedbackArea::Display, move || {
            crate::platform::display_command(pid, &command)
        });
    }
}

fn identity_key(identity: &Identity) -> (&'static str, &str) {
    match identity {
        Identity::Adb(serial) => ("adb", serial),
        Identity::UsbPeer(peer) => ("usb", peer.as_str()),
    }
}

fn presence(presence: Presence) -> &'static str {
    match presence {
        Presence::Connected => "Connected",
        Presence::Available => "Available to connect",
        Presence::Unknown => "Connection unconfirmed",
    }
}

fn add_display_devices(
    catalog: &mut Catalog,
    snapshot: &blent_config::display_control::Snapshot,
) -> Result<(), String> {
    snapshot.validate().map_err(|error| error.to_string())?;
    for assignment in &snapshot.assignments {
        let identity = assignment
            .target
            .identity()
            .map_err(|error| error.to_string())?;
        let device = Device::new(
            identity.clone(),
            identity.label(),
            Presence::Unknown,
            assignment.phase == blent_config::display_control::Phase::Assigned,
        )
        .map_err(|error| error.to_string())?;
        catalog.insert(device).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn resource_tab(resource: Resource) -> Tab {
    match resource {
        Resource::Display => Tab::Display,
        Resource::Camera => Tab::Camera,
        Resource::Microphone | Resource::Speakers => Tab::Audio,
    }
}

fn display_view(status: &Status, selected: &Identity) -> ResourceView {
    use blent_config::display_control::{Phase, Target};
    if !crate::capabilities().display || !status.daemon_running {
        return ResourceView::Unavailable;
    }
    let Some(snapshot) = &status.display else {
        return ResourceView::Here(Activity::Unknown);
    };
    let target = Target::from_identity(selected);
    let Some(assignment) = snapshot
        .assignments
        .iter()
        .find(|assignment| assignment.target == target)
    else {
        return ResourceView::NotStarted;
    };
    ResourceView::Here(match assignment.phase {
        Phase::Starting => Activity::Starting,
        Phase::Assigned => Activity::InUse,
        Phase::Stopping => Activity::Stopping,
    })
}

pub(super) fn set_target(
    selected: &Identity,
    serial: &mut Option<String>,
    peer: &mut Option<String>,
) {
    match selected {
        Identity::Adb(value) => {
            *serial = Some(value.clone());
            *peer = None;
        }
        Identity::UsbPeer(value) => {
            *serial = None;
            *peer = Some(value.as_str().to_owned());
        }
    }
}

pub(super) fn supports(transports: Transports, selected: &Identity) -> bool {
    match selected {
        Identity::Adb(_) => transports.adb,
        Identity::UsbPeer(_) => transports.usb,
    }
}

pub(super) fn title(resource: Resource) -> &'static str {
    match resource {
        Resource::Display => "Display & input",
        Resource::Camera => "Camera",
        Resource::Microphone => "Microphone",
        Resource::Speakers => "Speakers",
    }
}

fn activity(value: Activity) -> &'static str {
    match value {
        Activity::Unknown => "Activity unconfirmed",
        Activity::Off => "Stopped here",
        Activity::Starting => "Starting…",
        Activity::Waiting => "Waiting for tablet permission or video",
        Activity::InUse => "In use",
        Activity::Paused => "Waiting for host availability",
        Activity::Stopping => "Stopping · waiting for cleanup",
        Activity::Failed => "Unavailable · inspect settings",
    }
}

pub(super) fn label(view: ResourceView) -> String {
    match view {
        ResourceView::Unavailable => "Unavailable on this backend or connection".into(),
        ResourceView::NotStarted => "Not started here".into(),
        ResourceView::Here(value) => activity(value).into(),
        ResourceView::Elsewhere(value) => {
            format!("Reserved for another tablet · {}", activity(value))
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Action {
    Start,
    Stop,
    Configure,
}

pub(super) fn card(
    ui: &mut egui::Ui,
    resource: Resource,
    view: ResourceView,
    enabled: bool,
    error: Option<&str>,
) -> Option<Action> {
    let mut action = None;
    ui.push_id(title(resource), |ui| {
        egui::Frame::group(ui.style()).inner_margin(16.0).show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 2.0).max(0.0));
            ui.heading(title(resource));
            ui.label(if resource == Resource::Display && view == ResourceView::Here(Activity::InUse) { "Display assigned · video not verified".into() } else { label(view) });
            if let Some(error) = error {
                egui::ScrollArea::vertical().id_salt("resource-error").max_height(64.0).show(ui, |ui| { ui.colored_label(ui.visuals().error_fg_color, error); });
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                action = buttons(ui, resource, view, enabled);
            });
            if resource != Resource::Display {
                ui.small("Open Blent on tablet to allow this resource. Pairing grants no media access.");
            }
        });
    });
    action
}

fn buttons(
    ui: &mut egui::Ui,
    resource: Resource,
    view: ResourceView,
    enabled: bool,
) -> Option<Action> {
    let (verb, action, allowed) = if view.can_stop() {
        ("Stop", Action::Stop, true)
    } else {
        ("Start", Action::Start, view.can_start())
    };
    let mut selected = None;
    let color = if view.can_stop() {
        egui::Color32::from_rgb(145, 61, 61)
    } else {
        egui::Color32::from_rgb(38, 92, 165)
    };
    if ui
        .add_enabled(
            enabled && allowed,
            egui::Button::new(
                egui::RichText::new(format!("{verb} {}", title(resource).to_lowercase()))
                    .color(egui::Color32::WHITE),
            )
            .fill(color),
        )
        .clicked()
    {
        selected = Some(action);
    }
    if ui
        .button(format!("Configure {}", title(resource).to_lowercase()))
        .clicked()
    {
        selected = Some(Action::Configure);
    }
    selected
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
