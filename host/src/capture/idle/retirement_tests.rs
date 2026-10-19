//! T1117: original descriptors stay owned through failed cleanup observations.
use super::lease_tests::private_root;
use super::*;
use std::io;

#[test]
fn t1117_removal_faults_do_not_settle_before_the_original_namespace_is_clear() {
    let root = private_root();
    let fifo = root.path().join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Lease::open(&fifo, Default::default()).unwrap();
    lease.publish(500).unwrap();
    let mut attempts = 0;
    let mut pauses = 0;
    retire_request(
        || {
            attempts += 1;
            lease.clear_using(
                || {},
                |path| {
                    if attempts < 3 {
                        Err(io::Error::from(io::ErrorKind::PermissionDenied))
                    } else {
                        std::fs::remove_file(path)
                    }
                },
            )
        },
        || {
            pauses += 1;
        },
    );
    let left_owned_request = lease.path.exists();
    // Always settle this fixture before assertions, including the faulty version.
    lease.clear();
    drop(lease);
    assert_eq!(
        attempts, 3,
        "T1117 original retirement ignored failed removal"
    );
    assert_eq!(pauses, 2);
    assert!(!left_owned_request);
}

#[test]
fn t1117_numeric_original_cookie_is_not_disowned_by_leading_zero_encoding() {
    let root = private_root();
    let fifo = root.path().join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Lease::open(&fifo, Default::default()).unwrap();
    lease.publish(500).unwrap();
    let original = lease.contents().unwrap();
    let fields: Vec<_> = original.split_ascii_whitespace().collect();
    let alternate = format!(
        "0{} 0{} 0{} {} {}\n",
        fields[0], fields[1], fields[2], fields[3], fields[4]
    );
    std::fs::write(&lease.path, alternate).unwrap();
    lease.clear();
    let left_owned_request = lease.path.exists();
    // Restore canonical data so the failing implementation also retires cleanly.
    if left_owned_request {
        std::fs::write(&lease.path, original).unwrap();
    }
    drop(lease);
    assert!(
        !left_owned_request,
        "T1117 C accepts the same numeric cookie with leading zeroes"
    );
}

#[test]
fn t1117_bounded_cleanup_failures_require_each_obligation_to_settle() {
    for failures in 0..=256 {
        let mut attempts = 0;
        let mut pauses = 0;
        retire_request(
            || {
                attempts += 1;
                if attempts <= failures {
                    Err(io::Error::from(io::ErrorKind::Interrupted).into())
                } else {
                    Ok(())
                }
            },
            || {
                pauses += 1;
            },
        );
        assert_eq!(attempts, failures + 1);
        assert_eq!(pauses, failures);
    }
}

#[test]
fn t1117_bounded_cookie_values_and_unknown_storage_do_not_release_identity() {
    let root = private_root();
    let fifo = root.path().join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Lease::open(&fifo, Default::default()).unwrap();
    for padding in 0..=16 {
        let zeros = "0".repeat(padding);
        let text = format!(
            "{zeros}{} {zeros}{} {zeros}{} 1200 500\n",
            lease.identity.0, lease.identity.1, lease.token
        );
        assert_eq!(lease.owns_cookie(&text), Some(true));
    }
    for invalid in ["", "-1", "18446744073709551616", "NaN", "1.0", "\0"] {
        assert_eq!(lease.owns_cookie(&format!("{invalid} 1 1")), None);
        assert_eq!(lease.owns_cookie(&format!("1 {invalid} 1")), None);
        assert_eq!(lease.owns_cookie(&format!("1 1 {invalid}")), None);
    }
    assert_eq!(lease.owns_cookie("0 0 0"), Some(false));
    std::fs::write(&lease.path, [0xff]).unwrap();
    let unknown = lease.try_clear().is_err();
    let source = lease._inode.metadata().unwrap();
    std::fs::remove_file(&lease.path).unwrap();
    assert!(unknown);
    assert_eq!((source.dev(), source.ino()), lease.identity);
    assert!(lease.try_clear().is_ok());
}

#[tokio::test]
async fn t1117_original_worker_cleanup_receipt_waits_for_repaired_storage() {
    let root = private_root();
    let fifo = root.path().join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Arc::new(Lease::open(&fifo, Default::default()).unwrap());
    lease.publish(500).unwrap();
    let canonical = lease.contents().unwrap();
    let path = lease.path.clone();
    std::fs::write(&path, [0xff]).unwrap();
    let worker = worker::Worker::spawn(lease.stop.clone(), &Default::default(), move |holder| {
        holder.retain(lease)
    })
    .unwrap();
    let original = worker.completion();
    drop(worker);
    // Supporting native observation; the deterministic fault model above checks
    // every scripted failure independent of scheduling or this fixture window.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let retained = !original.is_complete();
    std::fs::write(&path, canonical).unwrap();
    while !original.is_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert!(
        retained,
        "T1117 actual original cleanup released unreadable storage"
    );
    assert!(!path.exists());
}
