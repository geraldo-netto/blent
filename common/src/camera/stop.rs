//! Scoped cutoff key. Withdrawal is not a native retirement acknowledgement.
use super::connection::validate_credential;
use anyhow::{ensure, Result};
use serde::Serialize;

#[derive(Serialize)]
pub struct Request<'a> {
    expected_token: &'a str,
    expected_port: u16,
}

impl<'a> Request<'a> {
    pub fn new(credential: &'a str, port: u16) -> Result<Self> {
        validate_credential(credential)?;
        ensure!(port != 0, "invalid camera stop port");
        Ok(Self {
            expected_token: credential,
            expected_port: port,
        })
    }

    pub fn credential(&self) -> &str {
        self.expected_token
    }

    pub fn port(&self) -> u16 {
        self.expected_port
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1204_shared_stop_key_retains_exact_original_credential_and_port() {
        let vector: serde_json::Value =
            serde_json::from_str(include_str!("../../../testdata/camera-stop.json")).unwrap();
        let token = vector["expected_token"].as_str().unwrap();
        let port = vector["expected_port"].as_u64().unwrap() as u16;
        let request = Request::new(token, port).unwrap();
        assert_eq!(request.credential(), token);
        assert_eq!(request.port(), port);
        assert_eq!(serde_json::to_value(request).unwrap(), vector);
    }
    #[test]
    fn t1204_bounded_invalid_stop_keys_never_become_requests() {
        let token = "ab".repeat(32);
        for port in 0..=1024 {
            assert_eq!(Request::new(&token, port).is_ok(), port != 0);
        }
        assert_eq!(Request::new(&token, u16::MAX).unwrap().port(), u16::MAX);
        for length in 0..=128 {
            assert_eq!(Request::new(&"a".repeat(length), 1).is_ok(), length == 64);
        }
        for byte in 0u8..=127 {
            let mut invalid = token.as_bytes().to_vec();
            invalid[32] = byte;
            let value = std::str::from_utf8(&invalid).unwrap();
            assert_eq!(
                Request::new(value, 1).is_ok(),
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            );
        }
        for value in ["é".repeat(32), "界".repeat(64)] {
            assert!(Request::new(&value, 1).is_err());
        }
    }
}
