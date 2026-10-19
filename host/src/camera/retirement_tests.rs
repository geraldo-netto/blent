//! T1126: the direct native workflow retains all declared original obligations.
use super::*;
use futures_util::FutureExt;

#[tokio::test]
async fn t1126_native_validation_failure_still_waits_for_original_report_obligations() {
    let report = tests::report();
    let original = blent_config::retirement::Pool::new(1)
        .unwrap()
        .reserve()
        .unwrap();
    report
        .reserve_native(crate::camera_control::NativeResource::Connection)
        .unwrap()
        .bind(original.completion());
    let mut options = tests::options();
    options.width = 0; // Validation fails before any OS device or ADB effects.
    let (_stop, stopped) = watch::channel(false);
    let mut work = Box::pin(run_native(options, stopped, report));
    let early = work.as_mut().now_or_never();
    let retained = early.is_none();
    drop(original);
    let result = match early {
        Some(result) => result,
        None => work.await,
    };
    assert!(result.is_err());
    assert!(
        retained,
        "T1126 direct native outcome escaped original Report obligations"
    );
}
