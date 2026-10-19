use super::*;
use crate::secure_connection::{client_config, server_config, IdentityRecord, PeerCertificate};
use std::sync::atomic::{AtomicBool, Ordering};

struct Owner(Arc<AtomicBool>);
impl Drop for Owner {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
fn identity() -> Identity {
    Identity::load(IdentityRecord::generate().unwrap()).unwrap()
}
fn retained() -> (RetainedIdentity, Arc<AtomicBool>) {
    let dropped = Arc::new(AtomicBool::new(false));
    let owner = Arc::new(Owner(dropped.clone()));
    (RetainedIdentity::new(identity(), owner), dropped)
}
fn peer() -> PeerCertificate {
    PeerCertificate::parse(identity().certificate().to_vec()).unwrap()
}

#[test]
fn t1102_config_key_and_selected_signer_each_retain_original_owner() {
    let (retained, dropped) = retained();
    let published = retained.shared();
    assert_eq!(retained.certificate(), published.certificate());
    let config = client_config(&published, &peer()).unwrap();
    let certified = config
        .client_auth_cert_resolver
        .resolve(&[], &[SignatureScheme::ECDSA_NISTP256_SHA256])
        .unwrap();
    let key = certified.key.clone();
    assert!(key.public_key().is_some());
    assert_eq!(key.algorithm(), SignatureAlgorithm::ECDSA);
    assert!(key.choose_scheme(&[]).is_none());
    assert_eq!(format!("{key:?}"), "RetainedSigningKey");
    let signer = key
        .choose_scheme(&[SignatureScheme::ECDSA_NISTP256_SHA256])
        .unwrap();
    assert_eq!(signer.scheme(), SignatureScheme::ECDSA_NISTP256_SHA256);
    assert_eq!(format!("{signer:?}"), "RetainedSigner");
    drop(retained);
    drop(published);
    assert!(!dropped.load(Ordering::Acquire));
    drop(config);
    drop(certified);
    assert!(!dropped.load(Ordering::Acquire));
    drop(key);
    assert!(!dropped.load(Ordering::Acquire));
    assert!(!signer
        .sign(b"T1102 original retained signer")
        .unwrap()
        .is_empty());
    drop(signer);
    assert!(dropped.load(Ordering::Acquire));
}

#[test]
fn t1102_server_configuration_owns_guard_after_identity_provider_ends() {
    let (retained, dropped) = retained();
    let published = retained.shared();
    let config = server_config(&published, &peer()).unwrap();
    drop(retained);
    drop(published);
    assert!(!dropped.load(Ordering::Acquire));
    drop(config);
    assert!(dropped.load(Ordering::Acquire));
}

#[derive(Debug)]
struct FailingKey;
impl SigningKey for FailingKey {
    fn choose_scheme(&self, offered: &[SignatureScheme]) -> Option<Box<dyn Signer>> {
        offered
            .contains(&SignatureScheme::ECDSA_NISTP256_SHA256)
            .then(|| Box::new(FailingSigner) as Box<dyn Signer>)
    }
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::ECDSA
    }
}
#[derive(Debug)]
struct FailingSigner;
impl Signer for FailingSigner {
    fn sign(&self, _message: &[u8]) -> Result<Vec<u8>, Error> {
        Err(Error::General("T1102 native signing rejected".into()))
    }
    fn scheme(&self) -> SignatureScheme {
        SignatureScheme::ECDSA_NISTP256_SHA256
    }
}

#[test]
fn t1102_rejection_optional_metadata_and_failed_signing_preserve_owner() {
    let dropped = Arc::new(AtomicBool::new(false));
    let key = RetainedKey {
        inner: Arc::new(FailingKey),
        owner: Arc::new(Owner(dropped.clone())),
    };
    assert!(key.public_key().is_none());
    assert_eq!(key.algorithm(), SignatureAlgorithm::ECDSA);
    assert!(key.choose_scheme(&[]).is_none());
    let signer = key
        .choose_scheme(&[SignatureScheme::ECDSA_NISTP256_SHA256])
        .unwrap();
    drop(key);
    assert_eq!(signer.scheme(), SignatureScheme::ECDSA_NISTP256_SHA256);
    assert_eq!(
        signer.sign(&[]).unwrap_err().to_string(),
        "unexpected error: T1102 native signing rejected"
    );
    assert!(!dropped.load(Ordering::Acquire));
    drop(signer);
    assert!(dropped.load(Ordering::Acquire));
}
