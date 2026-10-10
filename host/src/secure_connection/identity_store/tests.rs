use super::*;

#[derive(Default)]
struct Memory {
    bytes: Option<Vec<u8>>,
    reserved: bool,
    fail: u8,
}
impl IdentityStorage for Memory {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        ensure!(self.fail != 1 && !self.reserved, "load failed");
        Ok(self.bytes.clone())
    }
    fn reserve(&mut self) -> Result<()> {
        ensure!(self.fail != 2 && !self.reserved, "reserve failed");
        self.reserved = true;
        Ok(())
    }
    fn publish(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(self.fail != 3, "publish failed");
        self.bytes = Some(bytes.to_vec());
        self.reserved = false;
        Ok(())
    }
}
#[test]
fn t867_reuses_valid_identity_and_never_generates_after_storage_failure() {
    let mut store = Memory::default();
    let first = load_or_create(&mut store, IdentityRecord::generate).unwrap();
    let next = load_or_create(&mut store, || panic!("T867 unexpected rotation")).unwrap();
    assert_eq!(first.certificate(), next.certificate());
    for fail in [1, 2] {
        let mut store = Memory {
            fail,
            ..Memory::default()
        };
        assert!(
            load_or_create(&mut store, || panic!("T867 generate after storage failure")).is_err()
        );
    }
}
#[test]
fn t867_generation_and_publication_errors_preserve_pending_state() {
    let mut store = Memory {
        fail: 3,
        ..Memory::default()
    };
    assert!(load_or_create(&mut store, IdentityRecord::generate).is_err());
    assert!(store.reserved);
    assert!(load_or_create(&mut store, || panic!("T867 publication retry rotated key")).is_err());
    let mut store = Memory::default();
    assert!(load_or_create(&mut store, || anyhow::bail!("generation failed")).is_err());
    assert!(store.reserved);
}
#[test]
fn t867_invalid_record_or_crypto_does_not_rotate_or_publish() {
    let generated = IdentityRecord::generate().unwrap();
    let mut bytes = encode(&generated).unwrap();
    bytes[16] = 0;
    let mut store = Memory {
        bytes: Some(bytes.clone()),
        ..Memory::default()
    };
    assert!(load_or_create(&mut store, || panic!(
        "T867 corrupt persisted identity rotated"
    ))
    .is_err());
    assert_eq!(store.bytes, Some(bytes));
    let mut store = Memory::default();
    let invalid = IdentityRecord {
        certificate: generated.certificate,
        private_key: vec![0],
    };
    assert!(load_or_create(&mut store, || Ok(invalid)).is_err());
    assert!(store.bytes.is_none());
    assert!(store.reserved);
}
#[test]
fn t867_codec_rejects_truncation_trailing_overflow_and_bounded_mutations() {
    let record = IdentityRecord::generate().unwrap();
    let bytes = encode(&record).unwrap();
    assert_eq!(decode(&bytes).unwrap().private_key, record.private_key);
    for size in 0..bytes.len() {
        assert!(decode(&bytes[..size]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for index in 0..16 {
        let mut bad = bytes.clone();
        bad[index] ^= 0xff;
        assert!(decode(&bad).is_err());
    }
    for size in [0, MAX_PRIVATE_KEY + 1, u16::MAX as usize] {
        let invalid = IdentityRecord {
            certificate: vec![0; size],
            private_key: vec![0; size],
        };
        assert!(encode(&invalid).is_err());
        assert!(lengths(1, size).is_err());
    }
    t867_mutate_headers(&bytes);
}
fn t867_mutate_headers(bytes: &[u8]) {
    for seed in 0u32..4096 {
        let mut bad = bytes.to_vec();
        let value = seed.wrapping_mul(0x9e3779b9);
        bad[8..12].copy_from_slice(&value.to_be_bytes());
        assert!(decode(&bad).is_err());
    }
    assert!(decode(&vec![0; MAX_RECORD + 1]).is_err());
}
