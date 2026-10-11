//! Linux typed speaker request; generic media ownership and native setup are shared.
use crate::broker::linux::client::Launch;
use anyhow::{ensure, Result};
use blent_config::{
    audio::{AudioOptions, AudioProfile, Direction},
    connection::management::Profile,
};
use std::net::TcpListener;

pub(crate) type Bridge = crate::media_connection::Bridge<AudioProfile>;
pub(crate) fn native(
    options: &AudioOptions,
    token: &str,
    listener: TcpListener,
    launch: Launch,
    report: &crate::audio_control::Report,
) -> Result<Bridge> {
    options.validate()?;
    ensure!(
        options.profile.direction == Direction::Speakers,
        "direct-USB microphone is not implemented"
    );
    let (resource, profile) = options.profile.invitation_profile()?;
    let request =
        crate::media_request::request(options.target()?, resource, Profile::Audio(profile), token)?;
    let bridge =
        crate::media_connection::linux::native(&options.profile, token, request, listener, launch)?;
    report.retain(bridge.completion())?;
    Ok(bridge)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn t992_audio_usb_native_boundary_rejects_unavailable_direction_and_invalid_original_request(
    ) {
        let root = tempfile::tempdir().unwrap();
        let launch = Launch {
            executable: "/bin/false".into(),
            runtime: root.path().join("runtime"),
            state: root.path().join("state"),
            helper: root.path().join("missing-helper"),
        };
        let mut valid = AudioOptions::new(Direction::Speakers);
        valid.usb_peer = Some("ab".repeat(32));
        for invalid in 0..6 {
            let mut options = valid.clone();
            match invalid {
                0 => options.profile.direction = Direction::Microphone,
                1 => options.profile.buffer_ms = 21,
                2 => options.usb_peer = None,
                3 => options.serial = Some("original-adb".into()),
                4 => options.usb_peer = Some("short".into()),
                _ => (),
            }
            let token = if invalid == 5 {
                "short".into()
            } else {
                "ab".repeat(32)
            };
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            assert!(native(
                &options,
                &token,
                listener,
                launch.clone(),
                &crate::audio_control::Report::new(
                    tokio::sync::watch::channel(Default::default()).0
                )
            )
            .is_err());
        }
        assert_eq!(
            std::fs::read_dir(root.path()).unwrap().count(),
            0,
            "T992 invalid input started native work"
        );
    }
}
