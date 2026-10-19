//! A native identity owner follows every published key/configuration/signer.
//! Owners must be acyclic resource guards, with no back-reference to consumers.
use super::Identity;
use rustls::pki_types::SubjectPublicKeyInfoDer;
use rustls::sign::{Signer, SigningKey};
use rustls::{Error, SignatureAlgorithm, SignatureScheme};
use std::{fmt, sync::Arc};

/// Bind the original storage guard before sharing this identity. A pure memory
/// provider supplies an explicit unit owner; native providers supply their guard.
#[derive(Clone)]
pub struct RetainedIdentity(Arc<Identity>);
impl RetainedIdentity {
    pub fn new(mut identity: Identity, owner: Arc<dyn Send + Sync>) -> Self {
        identity.certified.key = Arc::new(RetainedKey {
            inner: identity.certified.key.clone(),
            owner,
        });
        Self(Arc::new(identity))
    }
    pub fn shared(&self) -> Arc<Identity> {
        self.0.clone()
    }
    pub fn certificate(&self) -> &[u8] {
        self.0.certificate()
    }
}

// Field order keeps the native guard alive during the inner object's destruction.
struct RetainedKey {
    inner: Arc<dyn SigningKey>,
    owner: Arc<dyn Send + Sync>,
}
impl fmt::Debug for RetainedKey {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("RetainedSigningKey")
    }
}
impl SigningKey for RetainedKey {
    fn choose_scheme(&self, offered: &[SignatureScheme]) -> Option<Box<dyn Signer>> {
        let inner = self.inner.choose_scheme(offered)?;
        Some(Box::new(RetainedSigner {
            inner,
            _owner: self.owner.clone(),
        }))
    }
    fn public_key(&self) -> Option<SubjectPublicKeyInfoDer<'_>> {
        self.inner.public_key()
    }
    fn algorithm(&self) -> SignatureAlgorithm {
        self.inner.algorithm()
    }
}
struct RetainedSigner {
    inner: Box<dyn Signer>,
    _owner: Arc<dyn Send + Sync>,
}
impl fmt::Debug for RetainedSigner {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("RetainedSigner")
    }
}
impl Signer for RetainedSigner {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        self.inner.sign(message)
    }
    fn scheme(&self) -> SignatureScheme {
        self.inner.scheme()
    }
}

#[cfg(test)]
mod tests;
