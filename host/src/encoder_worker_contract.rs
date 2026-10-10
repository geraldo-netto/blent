//! Private native-encoder protocol; independent of codecs, processes and OS APIs.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
pub const ARGUMENT: &str = "--blent-native-encoder-worker";
pub const MAX_JOB: usize = 8192;
pub const MAX_PACKET: usize = 8 * 1024 * 1024 - 4;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub encoder: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate: u32,
    pub quality: u32,
    pub slots: u32,
    pub workers: u32,
    pub device: String,
    pub nonce: u64,
    pub parent: u32,
}
impl Job {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (blent_config::MIN_FPS..=blent_config::MAX_FPS).contains(&self.fps),
            "Invalid encoder frame rate"
        );
        ensure!(
            (blent_config::MIN_BITRATE_KBPS..=blent_config::MAX_BITRATE_KBPS)
                .contains(&self.bitrate),
            "Invalid encoder bitrate"
        );
        ensure!(
            (blent_config::MIN_QUALITY..=blent_config::MAX_QUALITY).contains(&self.quality),
            "Invalid encoder quality"
        );
        blent_config::raw_frame::Layout::new(self.width, self.height, self.slots)?;
        blent_config::encoding::Profile::new(&self.encoder, self.fps, self.bitrate, self.quality)?
            .with_workers(self.workers)?;
        ensure!(
            self.nonce != 0 && self.parent != 0,
            "Missing encoder owner identity"
        );
        ensure!(
            self.device.len() <= 4096 && !self.device.contains('\0'),
            "Invalid encoder device path"
        );
        Ok(())
    }
    pub fn encode(&self) -> Result<String> {
        self.validate()?;
        let json = serde_json::to_string(self)?;
        ensure!(json.len() <= MAX_JOB, "Encoder job too large");
        Ok(json)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_JOB, "Encoder job too large");
        let job: Self = serde_json::from_slice(bytes)?;
        job.validate()?;
        Ok(job)
    }
}
pub fn packet_header(size: usize, key: bool) -> Result<[u8; 5]> {
    ensure!(
        (1..=MAX_PACKET).contains(&size),
        "Invalid encoded packet size"
    );
    let mut header = [0; 5];
    header[..4].copy_from_slice(&(size as u32).to_be_bytes());
    header[4] = u8::from(key);
    Ok(header)
}
pub fn packet_size(header: [u8; 5]) -> Result<(usize, bool)> {
    let size = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
    ensure!(header[4] <= 1, "Invalid encoded packet flags");
    packet_header(size, header[4] != 0)?;
    Ok((size, header[4] != 0))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn job() -> Job {
        Job {
            encoder: "libx264".into(),
            width: 64,
            height: 64,
            fps: 60,
            bitrate: 20000,
            quality: 20,
            slots: 4,
            workers: 1,
            device: "/dev/dri/renderD128".into(),
            nonce: 1,
            parent: 1,
        }
    }
    #[test]
    fn t819_worker_protocol_rejects_unbounded_or_invalid_jobs_and_packets() {
        let original = job();
        assert_eq!(
            Job::decode(original.encode().unwrap().as_bytes())
                .unwrap()
                .width,
            64
        );
        let invalid = [
            Job { width: 0, ..job() },
            Job {
                height: u32::MAX,
                ..job()
            },
            Job { fps: 0, ..job() },
            Job {
                quality: u32::MAX,
                ..job()
            },
            Job { slots: 9, ..job() },
            Job {
                workers: u32::MAX,
                ..job()
            },
            Job { nonce: 0, ..job() },
            Job { parent: 0, ..job() },
            Job {
                device: "a".repeat(4097),
                ..job()
            },
            Job {
                device: "a\0b".into(),
                ..job()
            },
            Job {
                bitrate: 0,
                ..job()
            },
            Job {
                bitrate: u32::MAX,
                ..job()
            },
            Job {
                fps: u32::MAX,
                ..job()
            },
        ];
        for (index, bad) in invalid.into_iter().enumerate() {
            assert!(bad.encode().is_err(), "index {index}");
        }
        assert!(Job::decode(&vec![b' '; MAX_JOB + 1]).is_err());
        assert!(Job::decode(b"{}").is_err());
        assert!(Job::decode(
            original
                .encode()
                .unwrap()
                .replace("\"width\":64", "\"unknown\":64")
                .as_bytes()
        )
        .is_err());
        for n in [1, 2, 4096, MAX_PACKET] {
            for key in [false, true] {
                assert_eq!(
                    packet_size(packet_header(n, key).unwrap()).unwrap(),
                    (n, key)
                );
            }
        }
        for n in [0, MAX_PACKET + 1, usize::MAX] {
            assert!(packet_header(n, false).is_err());
        }
        for byte in 2..=255 {
            assert!(packet_size([0, 0, 0, 1, byte]).is_err());
        }
        let mut seed = 91u32;
        for _ in 0..4096 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let mut bytes = [0; 5];
            bytes[..4].copy_from_slice(&seed.to_be_bytes());
            let result = packet_size(bytes);
            assert_eq!(result.is_ok(), (1..=MAX_PACKET).contains(&(seed as usize)));
        }
    }
}
