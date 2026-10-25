//! Typed audio invitation over the existing owned ADB reverse-map adapter.
use anyhow::{ensure, Context, Result};
use blent_config::audio::{
    connection::{Connection, Invitation, InvitationResult},
    AudioProfile,
};
use std::path::PathBuf;

pub(super) struct Bridge(crate::adb_mapping::Lease);
impl Bridge {
    pub(super) async fn create(
        adb: PathBuf,
        serial: Option<&str>,
        listener: &tokio::net::TcpListener,
        observe: &(dyn Fn(blent_config::retirement::Completion) -> Result<()> + Send + Sync),
    ) -> Result<Self> {
        Ok(Self(
            crate::adb_mapping::Lease::create(adb, serial, listener, observe).await?,
        ))
    }
    async fn broadcast(&self, token: &str, profile: &AudioProfile) -> Result<()> {
        self.0
            .shell(invitation(self.0.target(), token, profile)?)
            .await
            .context("Tablet did not accept audio request. Install updated Blent and open it.")
    }
}

fn invitation(
    target: &crate::adb_mapping::Target,
    token: &str,
    profile: &AudioProfile,
) -> Result<String> {
    profile.validate()?;
    ensure!(
        blent_config::connection::media::valid_credential(token),
        "invalid audio invitation credential"
    );
    let port = target.port()?;
    Ok(format!("am broadcast -n io.github.geraldo_netto.blent/com.blent.AudioReceiver --es token {token} --ei port {port} --ei direction {} --ei processing {} --ei buffer_ms {} --ez background {}\n", profile.direction as u8, profile.processing as u8, profile.buffer_ms, profile.background))
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
    use std::time::Duration;

    struct Fixture(crate::adb_mapping::Target);
    impl Fixture {
        async fn invite(&self, token: &str, profile: &AudioProfile) -> Result<()> {
            self.0
                .test_shell(
                    invitation(&self.0, token, profile)?,
                    "Tablet did not accept audio request",
                )
                .await
                .context("Tablet did not accept audio request")
        }
        async fn monitor(&self) -> Result<()> {
            std::future::pending().await
        }
    }
    #[tokio::test]
    async fn t992_audio_adb_invitation_rejects_raw_shell_values_before_native_work() {
        let root = tempfile::tempdir().unwrap();
        let bridge = Fixture(crate::adb_mapping::Target {
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
    #[tokio::test]
    async fn t1205_audio_requires_exact_single_current_broadcast_acknowledgement() {
        let root = tempfile::tempdir().unwrap();
        for (stdout, expected) in crate::adb_command::broadcast_tests::cases() {
            let bridge = Fixture(crate::adb_mapping::Target {
                adb: crate::adb_command::broadcast_tests::fixture(root.path(), &stdout, true),
                serial: "private-test".into(),
                remote: "tcp:34567".into(),
                local: "tcp:12345".into(),
            });
            let profile = AudioProfile::new(blent_config::audio::Direction::Speakers);
            let accepted = bridge.invite(&"ab".repeat(32), &profile).await.is_ok();
            assert_eq!(accepted, expected, "T1205 audio acknowledgement {stdout:?}");
        }
    }
    #[tokio::test]
    async fn t1205_audio_command_failure_cannot_grant_acceptance_or_leak_credential() {
        let root = tempfile::tempdir().unwrap();
        let bridge = Fixture(crate::adb_mapping::Target {
            adb: crate::adb_command::broadcast_tests::fixture(
                root.path(),
                b"Broadcast completed: result=1\n",
                false,
            ),
            serial: "private-test".into(),
            remote: "tcp:34567".into(),
            local: "tcp:12345".into(),
        });
        let credential = "ab".repeat(32);
        let profile = AudioProfile::new(blent_config::audio::Direction::Microphone);
        let error = bridge
            .invite(&credential, &profile)
            .await
            .unwrap_err()
            .to_string();
        assert!(!error.contains(&credential));
        assert!(error.contains("did not accept audio"));
    }
}
