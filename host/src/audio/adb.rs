//! Typed audio invitation over the existing owned ADB reverse-map adapter.
use anyhow::{ensure, Context, Result};
use blent_config::{
    audio::{
        connection::{Connection, Invitation, InvitationResult},
        AudioProfile,
    },
    commands::AsyncCommandExt,
};
use std::{path::PathBuf, time::Duration};
use tokio::process::Command;

pub(super) struct Bridge(crate::camera::bridge::Bridge);
impl Bridge {
    pub(super) async fn create(adb: PathBuf, serial: Option<&str>, port: u16) -> Result<Self> {
        Ok(Self(
            crate::camera::bridge::Bridge::create(adb, serial, port).await?,
        ))
    }
    async fn broadcast(&self, token: &str, profile: &AudioProfile) -> Result<()> {
        profile.validate()?;
        ensure!(
            blent_config::connection::media::valid_credential(token),
            "invalid audio invitation credential"
        );
        let port: u16 = self
            .0
            .remote
            .strip_prefix("tcp:")
            .context("invalid ADB audio port")?
            .parse()?;
        ensure!(port != 0, "invalid ADB audio port");
        let command = format!("am broadcast -n io.github.geraldo_netto.blent/com.blent.AudioReceiver --es token {token} --ei port {port} --ei direction {} --ei processing {} --ei buffer_ms {} --ez background {}\n", profile.direction as u8, profile.processing as u8, profile.buffer_ms, profile.background);
        let result = Command::new(&self.0.adb)
            .args(["-s", &self.0.serial, "shell"])
            .output_input_timeout(Some(command.as_bytes()), Duration::from_secs(5))
            .await?;
        ensure!(
            result.status.success() && String::from_utf8_lossy(&result.stdout).contains("result=1"),
            "Tablet did not accept audio request. Install updated Blent and open it."
        );
        Ok(())
    }
}
impl Invitation<AudioProfile> for Bridge {
    fn invite<'a>(&'a self, token: &'a str, profile: &'a AudioProfile) -> InvitationResult<'a> {
        Box::pin(self.broadcast(token, profile))
    }
}
impl Connection<AudioProfile> for Bridge {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(std::future::pending())
    }
    fn close(&self) -> InvitationResult<'_> {
        Box::pin(self.0.close())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn t992_audio_adb_invitation_rejects_raw_shell_values_before_native_work() {
        let root = tempfile::tempdir().unwrap();
        let bridge = Bridge(crate::camera::bridge::Bridge {
            adb: root.path().join("missing-adb"),
            serial: "original".into(),
            remote: "tcp:1234".into(),
            local: "tcp:4321".into(),
        });
        let mut profile = AudioProfile::new(blent_config::audio::Direction::Speakers);
        for token in ["short", "$(touch ignored)", "é"] {
            assert!(bridge
                .invite(token, &profile)
                .await
                .unwrap_err()
                .to_string()
                .contains("credential"));
        }
        profile.buffer_ms = 21;
        assert!(bridge.invite(&"ab".repeat(32), &profile).await.is_err());
        let monitor = bridge.monitor();
        assert!(tokio::time::timeout(Duration::from_millis(1), monitor)
            .await
            .is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
