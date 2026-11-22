use super::*;

fn stamp() -> Stamp {
    Stamp {
        device: 1,
        inode: 2,
        bytes: 3,
        mode: 0o100600,
        modified: (4, 5),
        changed: (6, 7),
    }
}

#[test]
fn t1493_memo_retains_only_current_identity_and_promotes_hits() {
    let mut memo = Memo::default();
    memo.remember("a".into(), stamp(), [1; 32], 2);
    memo.remember("b".into(), stamp(), [2; 32], 2);
    assert_eq!(memo.lookup(Path::new("a"), stamp(), 2), Some([1; 32]));
    memo.remember("c".into(), stamp(), [3; 32], 2);
    assert!(memo.lookup(Path::new("b"), stamp(), 2).is_none());
    memo.remember("a".into(), stamp(), [4; 32], 2);
    assert_eq!(memo.lookup(Path::new("a"), stamp(), 2), Some([4; 32]));
    assert_eq!(memo.0.len(), 2);
    let changed = Stamp {
        changed: (6, 8),
        ..stamp()
    };
    assert!(memo.lookup(Path::new("a"), changed, 2).is_none());
    assert!(memo.0.iter().all(|entry| entry.path != Path::new("a")));
}

#[test]
fn t1493_memo_bounds_eviction_and_disable_cover_every_admitted_capacity() {
    for limit in 0..=MAX_ENTRIES {
        let mut memo = Memo::default();
        for index in 0..=limit {
            memo.remember(index.to_string().into(), stamp(), [0; 32], limit);
            assert!(memo.0.len() <= limit);
            assert!(memo.0.capacity() <= MAX_ENTRIES);
        }
        assert_eq!(memo.0.len(), limit);
        assert!(memo.lookup(Path::new("missing"), stamp(), 0).is_none());
        assert!(memo.0.is_empty());
        assert_eq!(memo.0.capacity(), 0);
    }
}

#[test]
fn t1493_every_stat_component_invalidates_the_memo() {
    let original = stamp();
    for changed in [
        Stamp {
            device: 9,
            ..original
        },
        Stamp {
            inode: 9,
            ..original
        },
        Stamp {
            bytes: 9,
            ..original
        },
        Stamp {
            mode: 0o100400,
            ..original
        },
        Stamp {
            modified: (9, 5),
            ..original
        },
        Stamp {
            modified: (4, 9),
            ..original
        },
        Stamp {
            changed: (9, 7),
            ..original
        },
        Stamp {
            changed: (6, 9),
            ..original
        },
    ] {
        let mut memo = Memo::default();
        memo.remember("a".into(), original, [0; 32], 1);
        assert!(memo.lookup(Path::new("a"), changed, 1).is_none());
        assert!(memo.0.is_empty());
    }
}

#[test]
fn t1493_retained_digests_do_not_retain_native_files() {
    let root = tempfile::tempdir().unwrap();
    let limit = capacity().unwrap();
    let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
    for index in 0..=limit {
        let path = root.path().join(index.to_string());
        std::fs::write(&path, b"owned hash input").unwrap();
        assert!(super::super::file_hash(&path).is_some());
        std::fs::remove_file(path).unwrap();
    }
    assert!(cache().lock().unwrap().0.len() <= limit);
    assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), before);
}

#[test]
fn t1493_capacity_defaults_and_invalid_inputs_are_explicit() {
    if isolated() {
        return;
    }
    let key = "BLENT_RUNTIME_HASH_CACHE_ENTRIES";
    let file = tempfile::NamedTempFile::new().unwrap();
    std::env::remove_var(key);
    assert_eq!(capacity(), Some(64));
    for limit in 0..=MAX_ENTRIES {
        std::env::set_var(key, limit.to_string());
        assert_eq!(capacity(), Some(limit));
        assert!(super::super::file_hash(file.path()).is_some());
    }
    for invalid in ["", "-1", "1.5", "257", "18446744073709551616", "invalid"] {
        std::env::set_var(key, invalid);
        assert!(capacity().is_none());
        assert!(super::super::file_hash(file.path()).is_none());
    }
    use std::os::unix::ffi::OsStringExt;
    std::env::set_var(key, std::ffi::OsString::from_vec(vec![255]));
    assert!(capacity().is_none());
    assert!(super::super::file_hash(file.path()).is_none());
    std::env::set_var(key, "64");
    assert!(super::super::file_hash(file.path()).is_some());
}

fn isolated() -> bool {
    if std::env::var_os("BLENT_T1493_CAPACITY_TEST").is_some() {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "selection::worker::cache::context::runtime_identity::memo::tests::t1493_capacity_defaults_and_invalid_inputs_are_explicit", "--nocapture"])
        .env("BLENT_T1493_CAPACITY_TEST", "1").output().unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
