//! T923: bounded status snapshots and atomic assignment publication.
use super::*;
use std::process::{Child, Command};

struct LiveSnapshot {
    root: tempfile::TempDir,
    child: Child,
    snapshot: SessionSnapshot,
}
impl LiveSnapshot {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("blent");
        std::os::unix::fs::symlink("/bin/sleep", &executable).unwrap();
        let child = Command::new(executable).arg("30").spawn().unwrap();
        let snapshot = SessionSnapshot {
            pid: child.id(),
            start_ticks: process_start(child.id()).unwrap(),
            sessions: vec![session(0)],
        };
        Self {
            root,
            child,
            snapshot,
        }
    }
    fn path(&self) -> PathBuf {
        self.root.path().join("sessions.json")
    }
    fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.snapshot).unwrap()
    }
}
impl Drop for LiveSnapshot {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn session(instance: u32) -> TabletSession {
    TabletSession {
        identity: crate::tablets::Identity::Adb(format!("tablet-{instance}")),
        instance,
        video_port: 19000,
        input_port: 19100,
    }
}

#[test]
fn t923_reader_rejects_oversized_valid_json_at_exact_byte_boundary() {
    let fixture = LiveSnapshot::new();
    for length in [65_535, 65_536, 65_537, 131_072] {
        let mut bytes = fixture.bytes();
        bytes.resize(length, b' ');
        std::fs::write(fixture.path(), bytes).unwrap();
        assert_eq!(
            load_sessions(&fixture.path()).is_some(),
            length <= 65_536,
            "T923 snapshot length {length}"
        );
    }
}

#[test]
fn t923_reader_rejects_duplicate_and_excessive_records() {
    let mut fixture = LiveSnapshot::new();
    for count in 2..=8 {
        fixture.snapshot.sessions = vec![session(0); count];
        std::fs::write(fixture.path(), fixture.bytes()).unwrap();
        assert!(
            load_sessions(&fixture.path()).is_none(),
            "T923 count {count}"
        );
    }
}

#[test]
fn t923_publisher_rejects_invalid_batches_without_changing_file_or_state() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sessions.json");
    let mut ledger = SessionLedger::new(path.clone()).unwrap();
    ledger.update(vec![session(0)]).unwrap();
    let previous = std::fs::read(&path).unwrap();
    for count in 2..=8 {
        assert!(
            ledger.update(vec![session(0); count]).is_err(),
            "T923 count {count}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        assert_eq!(ledger.snapshot.sessions, vec![session(0)]);
    }
}

#[test]
fn t923_failed_publication_preserves_original_state_and_allows_retry() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sessions.json");
    let mut ledger = SessionLedger::new(path.clone()).unwrap();
    ledger.update(vec![session(0)]).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(ledger.update(vec![session(1)]).is_err());
    assert_eq!(ledger.snapshot.sessions, vec![session(0)]);
    std::fs::remove_dir(&path).unwrap();
    ledger.update(vec![session(1)]).unwrap();
    let snapshot: SessionSnapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(snapshot.sessions, vec![session(1)]);
}

#[test]
fn t923_reader_rejects_truncation_corruption_and_stale_owners() {
    let mut fixture = LiveSnapshot::new();
    let bytes = fixture.bytes();
    for length in 0..bytes.len() {
        std::fs::write(fixture.path(), &bytes[..length]).unwrap();
        assert!(
            load_sessions(&fixture.path()).is_none(),
            "T923 prefix {length}"
        );
    }
    for malformed in [b"null".as_slice(), b"{}", b"[]", b"\xff", b"{}{}"] {
        std::fs::write(fixture.path(), malformed).unwrap();
        assert!(load_sessions(&fixture.path()).is_none());
    }
    fixture.snapshot.start_ticks += 1;
    std::fs::write(fixture.path(), fixture.bytes()).unwrap();
    assert!(load_sessions(&fixture.path()).is_none());
}

#[test]
fn t923_unique_sparse_and_full_batches_round_trip_in_slot_order() {
    let fixture = LiveSnapshot::new();
    let mut ledger = SessionLedger::new(fixture.path()).unwrap();
    for mask in 0u32..16 {
        let mut sessions: Vec<_> = (0..4)
            .filter(|slot| mask & (1 << slot) != 0)
            .map(session)
            .collect();
        sessions.reverse();
        ledger.update(sessions.clone()).unwrap();
        sessions.reverse();
        assert_eq!(ledger.snapshot.sessions, sessions);
        // The publisher belongs to this test process; load only after assigning
        // the independently live fixture's PID/start-time owner.
        let mut snapshot: SessionSnapshot =
            serde_json::from_slice(&std::fs::read(fixture.path()).unwrap()).unwrap();
        snapshot.pid = fixture.snapshot.pid;
        snapshot.start_ticks = fixture.snapshot.start_ticks;
        std::fs::write(fixture.path(), serde_json::to_vec(&snapshot).unwrap()).unwrap();
        assert_eq!(load_sessions(&fixture.path()), Some(sessions));
    }
}

#[test]
fn t923_reader_and_publisher_reject_invalid_slots_and_ports() {
    let fixture = LiveSnapshot::new();
    let mut ledger = SessionLedger::new(fixture.path()).unwrap();
    for instance in [4, 5, 255, u32::MAX] {
        assert!(ledger.update(vec![session(instance)]).is_err());
    }
    for (video, input) in [(0, 19100), (19000, 0), (19000, 19000)] {
        let mut invalid = session(0);
        invalid.video_port = video;
        invalid.input_port = input;
        assert!(ledger.update(vec![invalid]).is_err());
        let value = serde_json::json!({"pid": fixture.snapshot.pid,
            "start_ticks": fixture.snapshot.start_ticks,
            "sessions": [{"serial": "tablet", "instance": 0,
                "video_port": video, "input_port": input}]});
        std::fs::write(fixture.path(), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(load_sessions(&fixture.path()).is_none());
    }
}

#[test]
fn t923_reader_rejects_fifth_record_before_decoding_its_fields() {
    let fixture = LiveSnapshot::new();
    let mut value: serde_json::Value = serde_json::from_slice(&fixture.bytes()).unwrap();
    let sessions = value["sessions"].as_array_mut().unwrap();
    sessions.clear();
    sessions.extend((0..4).map(|slot| serde_json::to_value(session(slot)).unwrap()));
    sessions.push(serde_json::json!({"not": "a display record"}));
    let error = serde_json::from_value::<SessionSnapshot>(value)
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("invalid length 5"),
        "T923: {error}"
    );
}

#[test]
fn t923_reader_caps_actual_reads_and_handles_io_failure() {
    struct Endless {
        consumed: usize,
    }
    impl Read for Endless {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.consumed += buffer.len();
            assert!(self.consumed <= 65_537, "T923 read exceeded byte bound");
            buffer.fill(b' ');
            Ok(buffer.len())
        }
    }
    let mut reader = Endless { consumed: 0 };
    assert!(read_snapshot(&mut reader).is_none());
    assert_eq!(reader.consumed, 65_537);
    assert!(load_sessions(&PathBuf::from("/definitely-absent-t923")).is_none());
    let root = tempfile::tempdir().unwrap();
    assert!(load_sessions(root.path()).is_none());
}

#[test]
fn t923_malformed_snapshot_cannot_remove_another_publishers_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("sessions.json");
    let ledger = SessionLedger::new(path.clone()).unwrap();
    let oversized = vec![b' '; 65_537];
    std::fs::write(&path, &oversized).unwrap();
    drop(ledger);
    assert_eq!(std::fs::read(&path).unwrap(), oversized);
}
