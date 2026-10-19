use super::*;
use crate::secure_connection::{Identity, IdentityRecord};

#[test]
fn t1103_backend_link_mapping_retains_original_snapshot_and_identity() {
    let identity =
        std::sync::Arc::new(Identity::load(IdentityRecord::generate().unwrap()).unwrap());
    let mut settings = Settings::new("/runtime".into(), "/state".into(), "/helper".into());
    let origin = Instant::now();
    assert_eq!(
        settings
            .link(identity.clone(), origin)
            .unwrap()
            .resource_timeout
            .milliseconds(),
        120_000
    );
    for milliseconds in [1, 37, 120_000, 600_000] {
        settings.resource_timeout_ms = milliseconds;
        let original = settings.link(identity.clone(), origin).unwrap();
        settings.resource_timeout_ms = 59;
        let future = settings.link(identity.clone(), origin).unwrap();
        assert_eq!(original.resource_timeout.milliseconds(), milliseconds);
        assert_eq!(future.resource_timeout.milliseconds(), 59);
        assert!(std::sync::Arc::ptr_eq(&original.identity, &identity));
        assert_eq!(original.origin, origin);
    }
    for value in [i64::MIN, -1, 0, 600_001, i64::MAX] {
        settings.resource_timeout_ms = value;
        assert!(settings.link(identity.clone(), origin).is_err());
    }
}
