//! T1086: isolated fake ADB only; no native tablet discovery or media devices.
use super::*;
use blent_config::{connection::media::Target, retirement::Completion};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

fn isolated() -> bool {
    if std::env::var_os("BLENT_T1086_PRIVATE").is_some() {
        return true;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "media_target::tests::t1086_original_native_target_and_cleanup",
            "--nocapture",
        ])
        .env("BLENT_T1086_PRIVATE", "1")
        .env("BLENT_COMMAND_STDOUT_BYTES", "1024")
        .env("BLENT_COMMAND_STDERR_BYTES", "1024")
        .env("BLENT_COMMAND_TOTAL_BYTES", "2048")
        .status()
        .unwrap();
    assert!(status.success(), "T1086 private native fixture failed");
    false
}

#[derive(Default)]
struct Proofs(Mutex<Vec<Completion>>);
impl Proofs {
    fn record(&self, proof: Completion) {
        let mut proofs = self.0.lock().unwrap();
        assert!(proofs.len() < 2, "T1086 command proof bound");
        proofs.push(proof);
    }
    async fn settle(&self) {
        while self
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|proof| !proof.is_complete())
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        self.0.lock().unwrap().clear();
    }
    fn count(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}
struct Fixture {
    root: tempfile::TempDir,
    adb: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let adb = root.path().join("adb");
        let body = format!(
            "#!/usr/bin/python3\nROOT = {}\n",
            serde_json::to_string(&root.path().to_string_lossy()).unwrap()
        );
        std::fs::write(
            &adb,
            body + r#"import json, pathlib, sys, time
root = pathlib.Path(ROOT)
args = sys.argv[1:]
config = json.loads((root / 'config.json').read_text())
with (root / 'calls.jsonl').open('a') as log:
    log.write(json.dumps(args) + '\n')
if args == ['get-serialno']:
    if config['mode'] == 'hold':
        (root / 'entered').write_text('entered')
        while True:
            time.sleep(.01)
    if config['mode'] == 'fail':
        print('unavailable original', file=sys.stderr)
        sys.exit(7)
    if config['mode'] == 'invalid':
        sys.stdout.buffer.write(b'\xff')
    else:
        print('' if config['mode'] == 'empty' else config['serial'])
elif args[:2] != ['-s', config['serial']]:
    print('original tablet absent', file=sys.stderr)
    sys.exit(8)
elif '--no-rebind' in args:
    pass
"#,
        )
        .unwrap();
        std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { root, adb }
    }
    fn config(&self, serial: &str, mode: &str) {
        std::fs::write(
            self.root.path().join("config.json"),
            serde_json::json!({"serial": serial, "mode": mode}).to_string(),
        )
        .unwrap();
    }
    fn calls(&self) -> Vec<Vec<String>> {
        std::fs::read_to_string(self.root.path().join("calls.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

#[tokio::test]
async fn t1086_original_native_target_and_cleanup() {
    if !isolated() {
        return;
    }
    let fixture = Fixture::new();
    std::env::set_var("PATH", fixture.root.path());
    let proofs = Proofs::default();
    let record = |proof| proofs.record(proof);
    original_and_replacement(&fixture, &proofs, &record).await;
    invalid_lookup_results(&fixture, &proofs, &record).await;
    bounded_output(&fixture, &proofs, &record).await;
    cancelled_lookup(&fixture, &proofs, &record).await;
}

async fn original_and_replacement(
    fixture: &Fixture,
    proofs: &Proofs,
    record: &(dyn Fn(Completion) + Send + Sync),
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    fixture.config("tablet-A", "ok");
    let intent = OriginalTarget::new(Target::Adb(None)).unwrap();
    assert_eq!(
        bind(&intent, record).await.unwrap().target(),
        Target::Adb(Some("tablet-A"))
    );
    proofs.settle().await;
    fixture.config("tablet-B", "ok");
    let bound = bind(&intent, record).await.unwrap();
    assert_eq!(bound.target(), Target::Adb(Some("tablet-A")));
    assert_eq!(
        proofs.count(),
        0,
        "T1086 cached intent performed another native lookup"
    );
    let Target::Adb(serial) = bound.target() else {
        panic!("T1086 changed transport")
    };
    assert!(crate::camera::bridge::Bridge::create(
        fixture.adb.clone(),
        serial,
        &listener,
        &|_| Ok(())
    )
    .await
    .is_err());
    let fresh = OriginalTarget::new(Target::Adb(None)).unwrap();
    let Target::Adb(serial) = bind(&fresh, record).await.unwrap().target() else {
        panic!("T1086 changed transport")
    };
    assert_eq!(serial, Some("tablet-B"));
    let bridge =
        crate::camera::bridge::Bridge::create(fixture.adb.clone(), serial, &listener, &|_| Ok(()))
            .await
            .unwrap();
    bridge.close().await.unwrap();
    proofs.settle().await;
    let calls = fixture.calls();
    assert_eq!(
        calls
            .iter()
            .filter(|args| args.as_slice() == ["get-serialno"])
            .count(),
        2
    );
    assert_eq!(calls[1], ["-s", "tablet-A", "reverse", "--list"]);
}

async fn invalid_lookup_results(
    fixture: &Fixture,
    proofs: &Proofs,
    record: &(dyn Fn(Completion) + Send + Sync),
) {
    for mode in ["empty", "fail", "invalid"] {
        fixture.config("unusable", mode);
        let intent = OriginalTarget::new(Target::Adb(None)).unwrap();
        assert!(bind(&intent, record).await.is_err());
        proofs.settle().await;
    }
    let absent = fixture.root.path().join("absent");
    std::env::set_var("PATH", &absent);
    let automatic = OriginalTarget::new(Target::Adb(None)).unwrap();
    assert!(bind(&automatic, record).await.is_err());
    let explicit = OriginalTarget::new(Target::Adb(Some(" exact serial; $(never) "))).unwrap();
    assert_eq!(
        bind(&explicit, record).await.unwrap().target(),
        Target::Adb(Some(" exact serial; $(never) "))
    );
    let usb = OriginalTarget::new(Target::parse(None, Some(&"ab".repeat(32))).unwrap()).unwrap();
    assert!(matches!(
        bind(&usb, record).await.unwrap().target(),
        Target::Usb(_)
    ));
    assert!(
        crate::adb_command::output_observed(&absent, &["get-serialno"], record)
            .await
            .is_err()
    );
    proofs.settle().await;
    std::env::set_var("PATH", fixture.root.path());
}

async fn entered(path: &Path) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn cancelled_lookup(
    fixture: &Fixture,
    proofs: &Proofs,
    record: &(dyn Fn(Completion) + Send + Sync),
) {
    fixture.config("late-A", "hold");
    let intent = OriginalTarget::new(Target::Adb(None)).unwrap();
    let mut pending = Box::pin(bind(&intent, record));
    let entry = fixture.root.path().join("entered");
    tokio::select! {
        result = &mut pending => panic!("T1086 blocked command completed: {}", result.is_ok()),
        _ = entered(&entry) => (),
    }
    assert_eq!(
        proofs.count(),
        2,
        "T1086 original child and collector proofs missing"
    );
    drop(pending);
    proofs.settle().await;
    fixture.config("first-effective-B", "ok");
    assert_eq!(
        bind(&intent, record).await.unwrap().target(),
        Target::Adb(Some("first-effective-B"))
    );
    proofs.settle().await;
}

async fn bounded_output(
    fixture: &Fixture,
    proofs: &Proofs,
    record: &(dyn Fn(Completion) + Send + Sync),
) {
    for limit in [2, 4, 16, 32] {
        std::env::set_var("BLENT_COMMAND_STDOUT_BYTES", limit.to_string());
        for length in [limit - 1, limit, limit + 1] {
            fixture.config(&"a".repeat(length), "ok");
            let intent = OriginalTarget::new(Target::Adb(None)).unwrap();
            // Fake ADB emits one newline in addition to the serial bytes.
            assert_eq!(bind(&intent, record).await.is_ok(), length < limit);
            proofs.settle().await;
        }
    }
    std::env::set_var("BLENT_COMMAND_STDOUT_BYTES", "1024");
    assert!(
        crate::adb_command::output_observed(&fixture.adb, &["\0"], record)
            .await
            .is_err()
    );
    proofs.settle().await;
}
