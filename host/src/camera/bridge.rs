//! A retained temporary mapping and a narrow borrowed invitation target.
use crate::adb_mapping;
pub use crate::adb_mapping::Target;
use anyhow::{ensure, Context, Result};
use blent_config::camera::{
    connection::{validate_credential, Invitation, InvitationResult},
    CameraProfile,
};
use std::path::PathBuf;

pub struct Bridge {
    lease: adb_mapping::Lease,
}

impl Bridge {
    pub async fn create(
        adb: PathBuf,
        serial: Option<&str>,
        listener: &tokio::net::TcpListener,
        observe: &(dyn Fn(blent_config::retirement::Completion) -> Result<()> + Send + Sync),
    ) -> Result<Self> {
        Ok(Self {
            lease: adb_mapping::Lease::create(adb, serial, listener, observe).await?,
        })
    }

    pub fn target(&self) -> &Target {
        self.lease.target()
    }

    pub async fn invite(&self, token: &str, options: &CameraProfile) -> Result<()> {
        self.lease
            .shell(invitation(self.target(), token, options)?)
            .await
            .context("ADB camera invitation failed")
    }

    pub async fn close(&self) -> Result<()> {
        self.lease.close().await
    }
}

fn invitation(target: &Target, token: &str, options: &CameraProfile) -> Result<String> {
    // Only hex and typed numbers enter the remote shell command. The serial
    // stays a distinct local argv element, including spaces/metacharacters.
    validate_credential(token)?;
    let capture = options.invitation_profile()?;
    ensure!(
        options.usb_peer.is_none(),
        "ADB camera invitation cannot target USB"
    );
    let port = target.port()?;
    let lens = capture.lens;
    Ok(format!(
            "am broadcast -n io.github.geraldo_netto.blent/com.blent.CameraReceiver --es token {token} --ei port {port} --ei width {} --ei height {} --ei fps {} --ei lens {lens} --ez background {} --ei bitrate {} --ei freshness_ms {} --ez adaptive_bitrate {} --ei min_bitrate {}\n",
            capture.width, capture.height, capture.fps, capture.background, capture.bitrate, capture.freshness_ms, capture.adaptive, capture.min_bitrate,
        ))
}

impl Target {
    #[cfg(test)]
    pub async fn invite(&self, token: &str, options: &CameraProfile) -> Result<()> {
        self.test_shell(
            invitation(self, token, options)?,
            "ADB camera invitation failed",
        )
        .await
    }
}

// Borrowed native-command fixtures exercise invitation encoding/ACK behavior.
// They do not create a mapping; production Connections always own a Lease.
#[cfg(test)]
impl Invitation<CameraProfile> for Target {
    fn invite<'a>(
        &'a self,
        credential: &'a str,
        profile: &'a CameraProfile,
    ) -> InvitationResult<'a> {
        Box::pin(Target::invite(self, credential, profile))
    }
}

#[cfg(test)]
impl blent_config::camera::connection::Connection<CameraProfile> for Target {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(std::future::pending())
    }
    fn close(&self) -> InvitationResult<'_> {
        Box::pin(std::future::ready(Ok(())))
    }
}

impl Invitation<CameraProfile> for Bridge {
    fn invite<'a>(
        &'a self,
        credential: &'a str,
        profile: &'a CameraProfile,
    ) -> InvitationResult<'a> {
        Box::pin(Bridge::invite(self, credential, profile))
    }
}

impl blent_config::camera::connection::Connection<CameraProfile> for Bridge {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(std::future::pending())
    }
    fn close(&self) -> InvitationResult<'_> {
        Box::pin(Bridge::close(self))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn t539_cleanup_preserves_other_reverse_mappings() {
        use blent_config::adb_reverse::temporary::{route, Route};
        assert_eq!(
            route(
                "Usb tcp:9999 tcp:1234\nUsb tcp:8890 tcp:8890",
                "tcp:9999",
                "tcp:1234"
            )
            .unwrap(),
            Route::Owned
        );
        assert_eq!(
            route("Usb tcp:9999 tcp:1235", "tcp:9999", "tcp:1234").unwrap(),
            Route::Foreign
        );
        for line in ["", "tcp:9999 tcp:1234", "Usb tcp:9999 tcp:1234 extra"] {
            assert_ne!(route(line, "tcp:9999", "tcp:1234").ok(), Some(Route::Owned));
        }
    }
}
