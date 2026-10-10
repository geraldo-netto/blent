//! Linux adapter: stock libusb runs only in a supervised opaque-byte helper.
use super::{Owner, Resource};
pub mod discovery;
use anyhow::{ensure, Context, Result};
use blent_config::{commands::OwnedChild, retirement::Completion};
use std::{future::Future, path::Path, pin::Pin, process::Stdio, time::Duration};
use tokio::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    bus: u8,
    ports: Vec<u8>,
}
impl Target {
    pub fn new(bus: u8, ports: &[u8]) -> Result<Self> {
        ensure!(
            bus > 0 && (1..=7).contains(&ports.len()) && ports.iter().all(|port| *port > 0),
            "invalid physical USB path"
        );
        Ok(Self {
            bus,
            ports: ports.to_vec(),
        })
    }
    fn arguments(&self) -> [String; 2] {
        [
            self.bus.to_string(),
            self.ports
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join("."),
        ]
    }
}
struct Process(OwnedChild);
impl Owner for Process {
    fn completion(&self) -> Completion {
        self.0.completion()
    }
    fn retire(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            let result = self.0.retire(Duration::ZERO).await;
            ensure!(
                result.completion.is_complete(),
                "USB worker retirement incomplete; port remains quarantined"
            );
            Ok(())
        })
    }
}
/// The helper is installed/configured locally, never selected by an incoming
/// peer message. Its inherited per-port lock survives until actual process exit.
/// No TLS state or application permission is implied by spawning this process.
pub fn open(helper: &Path, target: &Target) -> Result<Resource> {
    open_arguments(helper, &target.arguments())
}
/// Recheck observed metadata on the opened handle before any AOA transfer.
/// This binds a UI selection, never trust; TLS still proves the peer identity.
pub fn open_selected(helper: &Path, selected: &discovery::Candidate) -> Result<Resource> {
    ensure!(
        (1..=127).contains(&selected.address),
        "invalid selected USB address"
    );
    let mut arguments = vec!["--selected".to_owned()];
    arguments.extend(selected.target.arguments());
    arguments.extend([
        selected.address.to_string(),
        selected.vendor.to_string(),
        selected.product.to_string(),
    ]);
    open_arguments(helper, &arguments)
}
fn open_arguments(helper: &Path, arguments: &[String]) -> Result<Resource> {
    ensure!(
        helper.is_absolute(),
        "USB helper must use an absolute installed path"
    );
    let mut command = Command::new(helper);
    command
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = OwnedChild::spawn(&mut command).context("start USB I/O worker")?;
    let input = child.take_stdin().context("USB worker stdin unavailable")?;
    let output = child
        .take_stdout()
        .context("USB worker stdout unavailable")?;
    Ok(Resource {
        stream: Box::new(tokio::io::join(output, input)),
        owner: Box::new(Process(child)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[tokio::test]
    async fn t879_selected_snapshot_reaches_helper_as_bounded_exact_arguments() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let helper = root.path().join("selected-helper");
        std::fs::write(&helper, b"#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut selected = discovery::Candidate {
            target: Target::new(3, &[2, 4, 4]).unwrap(),
            address: 7,
            vendor: 6353,
            product: 201,
            device_class: 0,
            support: discovery::Support::Unknown,
        };
        for address in [0, 128, 255] {
            selected.address = address;
            assert!(open_selected(&helper, &selected).is_err());
        }
        selected.address = 7;
        assert!(open_selected(Path::new("relative"), &selected).is_err());
        let Resource { mut stream, owner } = open_selected(&helper, &selected).unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(3), stream.read_to_end(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bytes, b"--selected\n3\n2.4.4\n7\n6353\n201\n");
        drop(stream);
        owner.retire().await.unwrap();
    }
    #[test]
    fn t846_native_target_bounds_reject_before_spawning() {
        for (bus, ports) in [(0, vec![1]), (1, vec![]), (1, vec![0]), (1, vec![1; 8])] {
            assert!(Target::new(bus, &ports).is_err());
        }
        for bus in 1..=255 {
            let target = Target::new(bus, &[1, 2, 3, 4, 5, 6, 255]).unwrap();
            assert_eq!(
                target.arguments(),
                [bus.to_string(), "1.2.3.4.5.6.255".into()]
            );
        }
        let target = Target::new(1, &[1]).unwrap();
        assert!(open(Path::new("relative-helper"), &target).is_err());
        assert!(open(Path::new("/nonexistent/blent-test-usb-helper"), &target).is_err());
    }
    #[tokio::test]
    async fn t846_native_owner_duplex_and_actual_process_retirement() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let helper = temp.path().join("helper");
        // Fixture ignores selection arguments and echoes opaque bytes. No USB access.
        std::fs::write(&helper, b"#!/bin/sh\nexec cat\n").unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let Resource { mut stream, owner } =
            open(&helper, &Target::new(3, &[2, 4, 4]).unwrap()).unwrap();
        let completion = owner.completion();
        assert!(!completion.is_complete());
        stream.write_all(b"opaque transport bytes").await.unwrap();
        let mut echoed = [0; 22];
        tokio::time::timeout(Duration::from_secs(3), stream.read_exact(&mut echoed))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&echoed, b"opaque transport bytes");
        drop(stream);
        owner.retire().await.unwrap();
        assert!(completion.is_complete());
    }
}
