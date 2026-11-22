//! Navigation context owns one validated identity/name; it grants no media lease.
use super::*;
use blent_config::tablets::Identity;

pub(super) struct Target {
    identity: Identity,
    name: String,
}

impl Panel {
    pub(crate) fn configure_target(&mut self, identity: Identity, name: String) {
        self.configured = Some(Target { identity, name });
    }

    pub(crate) fn next_profile(&self, options: &CameraProfile) -> CameraProfile {
        let mut profile = options.clone();
        if let Some(target) = &self.configured {
            crate::resource_workspace::set_target(&target.identity, &mut profile.serial, &mut profile.usb_peer);
        }
        profile
    }

    pub(super) fn show_context(&self, ui: &mut egui::Ui, options: &CameraProfile, active: bool) {
        if active {
            if let Some(target) = self.owner.target() {
                ui.strong(format!("Active camera · {}", target.label()));
                ui.small("Live adjustments and Restart camera affect this active tablet.");
            }
        }
        match &self.configured {
            Some(target) => {
                ui.label(format!("Next camera start · {} · {}", target.name, route(&target.identity)));
                ui.small(format!("Tablet: {}", target.identity.label()));
            }
            None => { ui.label(format!("Next camera start · {}", saved_target(options))); }
        }
        ui.small("Image preferences are shared defaults. Choosing a tablet here does not change saved preferences.");
    }

    pub(crate) fn connection_controls(&mut self, ui: &mut egui::Ui, options: &mut CameraProfile) {
        if self.configured.is_some() {
            ui.label("Camera target");
            if ui.button("Use saved target").clicked() {
                self.configured = None;
            }
            ui.end_row();
            ui.label("");
            ui.label("Selected in Tablets. Stop the active camera before starting on a different tablet.");
            ui.end_row();
        } else {
            connection::show(ui, options, self.transports);
        }
    }
}

fn route(identity: &Identity) -> &'static str {
    match identity {
        Identity::Adb(_) => "USB debugging / network ADB",
        Identity::UsbPeer(_) => "Direct USB",
    }
}

fn saved_target(options: &CameraProfile) -> String {
    match options.target().map(crate::media_owner::identity) {
        Ok(Some(identity)) => format!("{} · {}", identity.label(), route(&identity)),
        Ok(None) => "Automatically choose one ADB tablet".into(),
        Err(_) => "Invalid saved target; check Advanced camera settings".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1789_context_never_retargets_admitted_camera() {
        let mut panel = Panel::default();
        let active = Identity::Adb("active".into());
        panel.owner.admit(Some(active.clone()), blent_config::media_start::Start::acknowledged());
        panel.configure_target(Identity::Adb("next".into()), "Next tablet".into());
        let profile = CameraProfile::default();
        assert_eq!(panel.admitted_profile(&profile).serial.as_deref(), Some("active"));
        assert_eq!(panel.next_profile(&profile).serial.as_deref(), Some("next"));
        assert_eq!(panel.target(), Some(&active));
        assert_eq!(profile, CameraProfile::default());
        let context = egui::Context::default();
        let _ = context.run(Default::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| panel.show_context(ui, &profile, true));
        });
    }

    #[test]
    fn t1789_saved_context_handles_auto_invalid_and_both_routes() {
        for profile in [CameraProfile::default(), CameraProfile { serial: Some("adb".into()), ..Default::default() }, CameraProfile { usb_peer: Some("ab".repeat(32)), ..Default::default() }, CameraProfile { usb_peer: Some("bad".into()), ..Default::default() }] {
            assert!(!saved_target(&profile).is_empty());
            let panel = Panel::default();
            assert_eq!(panel.next_profile(&profile), profile);
            let _ = egui::Context::default().run(Default::default(), |context| {
                egui::CentralPanel::default().show(context, |ui| panel.show_context(ui, &profile, false));
            });
        }
    }
}
