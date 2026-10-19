//! T1147: partial native output construction cannot retain its original file.
use super::*;

#[test]
fn t1147_partial_pair_failure_releases_original_file_before_reuse() {
    if private("camera::native_tests::admission::t1147_partial_pair_failure_releases_original_file_before_reuse") {
        fixture(|root, front, rear| runtime().block_on(partial(root, front, rear)));
    }
}
async fn partial(root: &std::path::Path, front: PathBuf, rear: PathBuf) {
    for rejected_rear in [root.join("missing"), front.clone()] {
        let mut options = tests::options();
        options.front_device = front.clone();
        options.rear_device = rejected_rear;
        let (_sender, mut stop) = watch::channel(false);
        let report = tests::new_report();
        let result = output_admission::admit(&options, &mut stop, &report).await;
        let rejected = result.is_err();
        drop(result);
        report.wait_native().await;
        drop(outputs::open_device(&front, "Blent Front").unwrap());
        drop(outputs::open_device(&rear, "Blent Rear").unwrap());
        assert!(rejected, "T1147 invalid rear endpoint was accepted");
    }
}
