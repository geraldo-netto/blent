//! Read-only USB inventory in a bounded, separately supervised native helper.
//! Descriptor observations identify candidates, never authenticated peers.
use super::Target;
use anyhow::{ensure, Context, Result};
use blent_config::{commands::OwnedChild, retirement::Completion};
use std::{collections::BTreeSet, path::Path, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::ChildStdout, time::Instant};

const HEADER: usize = 16;
const RECORD: usize = 16;
pub const MAX_DEVICES: usize = 512;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub devices: usize,
    pub timeout: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            devices: 64,
            timeout: Duration::from_secs(5),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<()> {
        ensure!(
            (1..=MAX_DEVICES).contains(&self.devices),
            "invalid discovery capacity"
        );
        ensure!(
            !self.timeout.is_zero() && self.timeout <= Duration::from_secs(60),
            "invalid discovery timeout"
        );
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    /// No active AOA query was made. Vendor/product observations cannot prove it.
    Unknown,
    /// Observed stock accessory VID/PID, still requiring Hello and mutual TLS.
    Accessory,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub target: Target,
    pub address: u8,
    pub vendor: u16,
    pub product: u16,
    pub device_class: u8,
    pub support: Support,
}
impl Candidate {
    /// Local display hint only, not an identity or a command argument.
    pub fn physical_path(&self) -> (u8, &[u8]) {
        (self.target.bus, &self.target.ports)
    }
}

pub struct Scan {
    child: OwnedChild,
    output: ChildStdout,
    completion: Completion,
    limits: Limits,
    deadline: Instant,
}
impl Scan {
    pub fn start(helper: &Path, limits: Limits) -> Result<Self> {
        limits.validate()?;
        ensure!(
            helper.is_absolute(),
            "discovery helper must be an installed absolute path"
        );
        let deadline = Instant::now() + limits.timeout;
        let mut command = tokio::process::Command::new(helper);
        command
            .args(["--list", &limits.devices.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = OwnedChild::spawn(&mut command).context("start USB inventory helper")?;
        let completion = child.completion();
        let output = child
            .take_stdout()
            .context("inventory helper stdout unavailable")?;
        Ok(Self {
            child,
            output,
            completion,
            limits,
            deadline,
        })
    }

    /// Retain this independently before finish/cancellation. A timed out helper
    /// can remain quarantined; timeout is never native retirement evidence.
    pub fn completion(&self) -> Completion {
        self.completion.clone()
    }

    /// The single deadline covers collection, process exit, and attempted
    /// retirement. Failure preserves the existing OwnedChild reaper admission.
    pub async fn finish(self) -> Result<Vec<Candidate>> {
        let Self {
            mut child,
            mut output,
            completion,
            limits,
            deadline,
        } = self;
        let operation = async {
            let bytes = collect(&mut output, HEADER + limits.devices * RECORD).await?;
            ensure!(child.wait().await?.success(), "USB inventory helper failed");
            decode(&bytes, limits.devices)
        };
        let result = tokio::time::timeout_at(deadline, operation)
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("USB inventory deadline expired")));
        drop(output);
        if result.is_err() {
            let _ = tokio::time::timeout_at(deadline, child.retire(Duration::ZERO)).await;
        } else {
            drop(child);
            ensure!(completion.is_complete(), "USB inventory helper still owned");
        }
        result
    }
}

async fn collect(output: &mut ChildStdout, maximum: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0; maximum + 1];
    let mut length = 0;
    loop {
        let count = output.read(&mut bytes[length..]).await?;
        if count == 0 {
            break;
        }
        length += count;
        ensure!(length <= maximum, "USB inventory output exceeds capacity");
    }
    bytes.truncate(length);
    Ok(bytes)
}

fn decode(bytes: &[u8], capacity: usize) -> Result<Vec<Candidate>> {
    ensure!(
        (1..=MAX_DEVICES).contains(&capacity),
        "invalid inventory decode capacity"
    );
    ensure!(
        bytes.len() >= HEADER && &bytes[..8] == b"BLUI\0\0\0\x01",
        "invalid USB inventory header"
    );
    let count = u16::from_be_bytes([bytes[8], bytes[9]]) as usize;
    ensure!(
        count <= capacity && bytes.len() == HEADER + count * RECORD,
        "invalid USB inventory extent"
    );
    ensure!(
        bytes[10..HEADER].iter().all(|byte| *byte == 0),
        "invalid USB inventory reserved bytes"
    );
    let mut candidates = Vec::with_capacity(count);
    let mut targets = BTreeSet::new();
    for record in bytes[HEADER..].as_chunks::<RECORD>().0 {
        let decoded = candidate(record)?;
        ensure!(
            targets.insert((decoded.target.bus, decoded.target.ports.clone())),
            "duplicate USB physical target"
        );
        candidates.push(decoded);
    }
    Ok(candidates)
}

fn candidate(bytes: &[u8]) -> Result<Candidate> {
    let depth = bytes[2] as usize;
    ensure!((1..=7).contains(&depth), "invalid USB port depth");
    ensure!(
        (1..=127).contains(&bytes[1]) && bytes[3] != 9,
        "invalid peripheral address/class"
    );
    ensure!(
        bytes[8 + depth..].iter().all(|byte| *byte == 0),
        "invalid USB record padding"
    );
    let target = Target::new(bytes[0], &bytes[8..8 + depth])?;
    let vendor = u16::from_be_bytes([bytes[4], bytes[5]]);
    let product = u16::from_be_bytes([bytes[6], bytes[7]]);
    let support = if vendor == 0x18d1 && matches!(product, 0x2d00 | 0x2d01) {
        Support::Accessory
    } else {
        Support::Unknown
    };
    Ok(Candidate {
        target,
        address: bytes[1],
        vendor,
        product,
        device_class: bytes[3],
        support,
    })
}

#[cfg(test)]
mod tests;
