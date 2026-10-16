//! Optional native capture boundary; policy and process ownership stay separate.
use anyhow::Result;
use blent_config::{
    capture_activity::Request, commands::OwnedChild, linux::capture_control::Control,
};
use tokio::process::Command;

#[derive(Default)]
pub(super) struct Channel {
    enabled: bool,
    control: Option<Control>,
}
impl Channel {
    pub(super) fn enable(&mut self) {
        self.enabled = true;
    }
    pub(super) fn attach(&mut self, command: &mut Command) -> Result<()> {
        self.control = None;
        if self.enabled {
            let (control, inherited) = Control::pair()?;
            inherited.attach(command)?;
            self.control = Some(control);
        }
        Ok(())
    }
    pub(super) fn retain(&self, child: &mut OwnedChild) {
        if let Some(control) = &self.control {
            child.keep_alive(control.lease());
        }
    }
    pub(super) async fn set_active(&mut self, active: bool) -> Result<Request> {
        use anyhow::Context;
        self.control
            .as_mut()
            .context("capture activity control unavailable")?
            .set_active(active)
            .await
    }
    pub(super) fn disconnect(&mut self) {
        self.control = None;
    }
}
