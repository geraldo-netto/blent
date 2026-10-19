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

#[test]
fn t1105_backend_link_mapping_retains_original_tls_budget_separate_from_other_stages() {
    let identity =
        std::sync::Arc::new(Identity::load(IdentityRecord::generate().unwrap()).unwrap());
    let mut settings = Settings::new("/runtime".into(), "/state".into(), "/helper".into());
    let origin = Instant::now();
    assert_eq!(
        settings
            .link(identity.clone(), origin)
            .unwrap()
            .tls_timeout
            .duration(),
        Duration::from_secs(10)
    );
    for milliseconds in [1, 37, 10000, 60000] {
        settings.tls_timeout = Duration::from_millis(milliseconds);
        let original = settings.link(identity.clone(), origin).unwrap();
        settings.tls_timeout = Duration::from_millis(17);
        let future = settings.link(identity.clone(), origin).unwrap();
        assert_eq!(
            original.tls_timeout.duration(),
            Duration::from_millis(milliseconds)
        );
        assert_eq!(future.tls_timeout.duration(), Duration::from_millis(17));
        assert_eq!(original.approval_timeout, Duration::from_secs(120));
        assert_eq!(original.setup_timeout, Duration::from_secs(60));
        assert_eq!(original.limits.io_timeout, Duration::from_secs(5));
        assert_eq!(original.resource_timeout.milliseconds(), 120000);
        assert!(std::sync::Arc::ptr_eq(&original.identity, &identity));
    }
    for value in [
        Duration::ZERO,
        Duration::from_secs(60) + Duration::from_nanos(1),
        Duration::MAX,
    ] {
        settings.tls_timeout = value;
        assert!(settings.link(identity.clone(), origin).is_err());
    }
}
mod shared;
