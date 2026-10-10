use super::*;
use std::os::unix::fs::PermissionsExt;

fn bytes(records: &[[u8; RECORD]]) -> Vec<u8> {
    let mut bytes = b"BLUI\0\0\0\x01".to_vec();
    bytes.extend_from_slice(&(records.len() as u16).to_be_bytes());
    bytes.extend_from_slice(&[0; 6]);
    for record in records {
        bytes.extend_from_slice(record);
    }
    bytes
}
fn record(port: u8) -> [u8; RECORD] {
    [1, 2, 1, 0, 0x18, 0xd1, 0x2d, 0, port, 0, 0, 0, 0, 0, 0, 0]
}
fn fixture(data: &[u8], operation: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let helper = directory.path().join("helper");
    std::fs::write(directory.path().join("helper.data"), data).unwrap();
    let script = format!(
        "#!/bin/sh\n[ \"$1\" = --list ] || exit 77\nprintf '%s' \"$$\" > \"$0.pid\"\n{operation}\n"
    );
    std::fs::write(&helper, script).unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    (directory, helper)
}
async fn retired(completion: &Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
fn assert_exited(helper: &Path) {
    let pid: i32 = std::fs::read_to_string(helper.with_extension("pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

#[test]
fn t876_fixed_inventory_validates_candidates_and_unknown_support() {
    let mut unknown = record(1);
    unknown[4..8].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
    let mut adb_accessory = record(3);
    adb_accessory[7] = 1;
    let candidates = decode(&bytes(&[unknown, record(2), adb_accessory]), 3).unwrap();
    assert_eq!(candidates[0].support, Support::Unknown);
    assert_eq!(candidates[0].physical_path(), (1, &[1][..]));
    assert_eq!(candidates[0].vendor, 0x1234);
    assert_eq!(candidates[0].product, 0x5678);
    assert_eq!(candidates[1].support, Support::Accessory);
    assert_eq!(candidates[2].support, Support::Accessory);
    assert!(decode(&bytes(&[]), 1).unwrap().is_empty());
    let limits = Limits::default();
    assert_eq!(limits.devices, 64);
    assert_eq!(limits.timeout, Duration::from_secs(5));
}

#[test]
fn t876_inventory_extent_metadata_and_duplicate_rejections() {
    let valid = bytes(&[record(1)]);
    for length in 0..valid.len() {
        assert!(decode(&valid[..length], 1).is_err());
    }
    for capacity in [0, 513, usize::MAX] {
        assert!(decode(&valid, capacity).is_err());
    }
    assert!(decode(&bytes(&[record(1), record(1)]), 2).is_err());
    assert!(decode(&bytes(&[record(1), record(2)]), 1).is_err());
    for (offset, value) in [
        (0, 0),
        (4, 1),
        (10, 1),
        (16, 0),
        (17, 128),
        (18, 0),
        (18, 8),
        (19, 9),
        (24, 0),
        (31, 1),
    ] {
        let mut invalid = valid.clone();
        invalid[offset] = value;
        assert!(decode(&invalid, 1).is_err(), "offset={offset}");
    }
    let mut extra = valid.clone();
    extra.push(0);
    assert!(decode(&extra, 1).is_err());
}

#[test]
fn t876_bounded_mutations_and_maximum_inventory_never_escape_limits() {
    let records: Vec<_> = (0..MAX_DEVICES)
        .map(|index| {
            let mut record = record((index % 255 + 1) as u8);
            record[0] = (index / 255 + 1) as u8;
            record
        })
        .collect();
    assert_eq!(
        decode(&bytes(&records), MAX_DEVICES).unwrap().len(),
        MAX_DEVICES
    );
    let valid = bytes(&[record(1)]);
    let mut random = 876u64;
    for _ in 0..4096 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut mutated = valid.clone();
        mutated[random as usize % valid.len()] = (random >> 32) as u8;
        if let Ok(candidates) = decode(&mutated, 1) {
            assert_eq!(candidates.len(), 1);
            assert!((1..=127).contains(&candidates[0].address));
            assert!((1..=7).contains(&candidates[0].physical_path().1.len()));
        }
    }
}

#[tokio::test]
async fn t876_supervised_inventory_requires_valid_output_and_actual_exit() {
    let (_directory, helper) = fixture(&bytes(&[record(1)]), "exec cat \"$0.data\"");
    let scan = Scan::start(&helper, Limits::default()).unwrap();
    let completion = scan.completion();
    assert!(!completion.is_complete());
    let candidates = scan.finish().await.unwrap();
    assert_eq!(candidates.len(), 1);
    assert!(completion.is_complete());
    assert_exited(&helper);
}

async fn rejected(data: &[u8], operation: &str) {
    let (_directory, helper) = fixture(data, operation);
    let scan = Scan::start(
        &helper,
        Limits {
            devices: 1,
            timeout: Duration::from_secs(2),
        },
    )
    .unwrap();
    let completion = scan.completion();
    assert!(scan.finish().await.is_err());
    retired(&completion).await;
    assert_exited(&helper);
}

#[tokio::test]
async fn t876_output_overflow_malformed_record_and_failed_exit_retire_native_helper() {
    rejected(&[0; HEADER + RECORD + 1], "exec cat \"$0.data\"").await;
    rejected(b"bad", "exec cat \"$0.data\"").await;
    rejected(&[], "exit 7").await;
}

#[tokio::test]
async fn t876_timeout_and_cancel_preserve_completion_until_actual_exit() {
    let (_directory, helper) = fixture(&[], "exec sleep 60");
    let scan = Scan::start(
        &helper,
        Limits {
            devices: 1,
            timeout: Duration::from_millis(30),
        },
    )
    .unwrap();
    let completion = scan.completion();
    assert!(tokio::time::timeout(Duration::from_secs(2), scan.finish())
        .await
        .unwrap()
        .is_err());
    retired(&completion).await;
    assert_exited(&helper);
    let scan = Scan::start(&helper, Limits::default()).unwrap();
    let completion = scan.completion();
    assert!(
        tokio::time::timeout(Duration::from_millis(30), scan.finish())
            .await
            .is_err()
    );
    retired(&completion).await;
    assert_exited(&helper);
}

#[tokio::test]
async fn t876_invalid_scan_configuration_never_starts_native_work() {
    let (_directory, helper) = fixture(&[], "exit 0");
    for devices in [0, 513, usize::MAX] {
        assert!(Scan::start(
            &helper,
            Limits {
                devices,
                ..Limits::default()
            }
        )
        .is_err());
    }
    for timeout in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        assert!(Scan::start(
            &helper,
            Limits {
                timeout,
                ..Limits::default()
            }
        )
        .is_err());
    }
    assert!(!helper.with_extension("pid").exists());
    assert!(Scan::start(Path::new("relative"), Limits::default()).is_err());
    assert!(Scan::start(&helper.with_extension("absent"), Limits::default()).is_err());
}
