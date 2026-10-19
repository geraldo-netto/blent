//! T1125: activity scopes cannot bypass the original attempt's cutoff.
use super::*;

#[test]
fn t1125_nested_scope_preserves_original_cutoff_and_inflight_reservation() {
    let report = Report::new(watch::channel(State::Stopped).0, watch::channel(None).0);
    let original = crate::report_scope::Owner::default();
    original.begin();
    let attempt = report.original(original.permit().unwrap());
    let activity = crate::report_scope::Owner::default();
    activity.begin();
    let nested = attempt.scoped(activity.permit().unwrap());
    nested.update(State::Streaming);
    assert_eq!(report.state(), State::Streaming);
    let admitted = nested.reserve_native(native::Resource::Connection).unwrap();
    original.revoke();
    nested.update(State::Stopped);
    assert_eq!(
        report.state(),
        State::Streaming,
        "T1125 nested scope bypassed original cutoff"
    );
    // An already admitted synchronous creator still has its own funded slot.
    assert!(!report.native_retired());
    drop(admitted);
    assert!(report.native_retired());
    activity.begin();
    let later = attempt.scoped(activity.permit().unwrap());
    assert!(later.reserve_native(native::Resource::Connection).is_err());
    later.update(State::Failed("stale activity".into()));
    assert_eq!(report.state(), State::Streaming);
}
