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
            let (control, inherited) = Control::pair(ack_budget()?)?;
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

fn ack_budget() -> Result<std::time::Duration> {
    use anyhow::Context;
    let milliseconds =
        blent_config::retirement_limits::configured("BLENT_CAPTURE_ACK_MS", 2000, 60_000)
            .context("Invalid BLENT_CAPTURE_ACK_MS")?;
    Ok(std::time::Duration::from_millis(milliseconds as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn t1686_ack_budget_defaults_and_invalid_bounds_are_explicit() {
        if crate::capture::ownership_tests::isolated("capture::helper::activity::tests::t1686_ack_budget_defaults_and_invalid_bounds_are_explicit") { return; }
        std::env::remove_var("BLENT_CAPTURE_ACK_MS");
        assert_eq!(ack_budget().unwrap(), std::time::Duration::from_secs(2));
        for value in [
            "",
            "0",
            "-1",
            "1.5",
            "60001",
            "18446744073709551616",
            "not-a-number",
        ] {
            std::env::set_var("BLENT_CAPTURE_ACK_MS", value);
            assert!(ack_budget()
                .unwrap_err()
                .to_string()
                .contains("BLENT_CAPTURE_ACK_MS"));
            let mut channel = Channel::default();
            channel.enable();
            assert!(channel.attach(&mut Command::new("/never/spawned")).is_err());
            assert!(channel.control.is_none());
        }
        for milliseconds in [1, 50, 60_000] {
            std::env::set_var("BLENT_CAPTURE_ACK_MS", milliseconds.to_string());
            assert_eq!(
                ack_budget().unwrap(),
                std::time::Duration::from_millis(milliseconds)
            );
            let mut channel = Channel::default();
            channel.enable();
            channel.attach(&mut Command::new("/never/spawned")).unwrap();
            assert!(channel.control.is_some());
            channel.disconnect();
        }
    }
}
