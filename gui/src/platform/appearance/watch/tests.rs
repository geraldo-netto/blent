use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
const FIXTURE: &str = include_str!("../fixture.toml");

#[tokio::test]
async fn t1795_frames_reject_invalid_size_truncation_and_preserve_next() {
    for size in [0u32, 8193, u32::MAX] {
        assert!(frame(&mut size.to_be_bytes().as_slice()).await.is_err());
    }
    for size in [1u32, 8192] {
        let mut input = size.to_be_bytes().to_vec();
        input.extend(vec![7; size as usize]);
        input.push(9);
        let mut reader = input.as_slice();
        assert_eq!(frame(&mut reader).await.unwrap(), vec![7; size as usize]);
        assert_eq!(reader, [9]);
        assert!(frame(&mut &input[..input.len() - 2]).await.is_err());
    }
}
#[test]
fn t1795_latest_update_replaces_pending_value_with_one_wake() {
    let latest = Latest::default();
    let wakes = AtomicUsize::new(0);
    let notify = || {
        wakes.fetch_add(1, Ordering::Relaxed);
    };
    assert!(latest.take().is_none());
    for index in 0..100 {
        latest.publish(Err(index.to_string()), &notify);
    }
    assert_eq!(wakes.load(Ordering::Relaxed), 1);
    assert_eq!(latest.take().unwrap().unwrap_err(), "99");
    latest.publish(Err("next".into()), &notify);
    assert_eq!(wakes.load(Ordering::Relaxed), 2);
}
fn completion(worker: &blent::control_workers::Worker) -> blent_config::retirement::Completion {
    worker.completion()
}
fn settled(done: blent_config::retirement::Completion) {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while !done.is_complete() {
        assert!(std::time::Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn t1795_spawn_rejection_is_reported_and_original_worker_joins() {
    let (notify, notified) = std::sync::mpsc::channel();
    let watch = Watch::start_command(Command::new("/nonexistent/blent-t1795"), move || {
        let _ = notify.send(());
    })
    .unwrap();
    notified.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(watch.take().unwrap().is_err());
    let done = completion(&watch._worker);
    drop(watch);
    settled(done);
}
#[test]
fn t1795_native_stream_coalesces_identical_updates_and_cancels_original_child() {
    let script = "import sys,struct; data=sys.argv[1].encode(); sys.stdout.buffer.write((struct.pack('>I',len(data))+data)*8); sys.stdout.buffer.flush(); sys.stdin.buffer.read();";
    let mut command = Command::new("/usr/bin/python3");
    command.args(["-I", "-c", script, FIXTURE]);
    let (notify, notified) = std::sync::mpsc::channel();
    let watch = Watch::start_command(command, move || {
        let _ = notify.send(());
    })
    .unwrap();
    notified.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!watch.take().unwrap().unwrap().1.is_empty());
    assert!(notified.recv_timeout(Duration::from_millis(200)).is_err());
    let native = watch
        .latest
        .native
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    assert!(!native.is_complete());
    let done = completion(&watch._worker);
    drop(watch);
    settled(done);
    settled(native);
}
#[tokio::test]
async fn t1795_invalid_snapshot_preserves_subscription_and_recovers_on_valid_change() {
    let mut input = 3u32.to_be_bytes().to_vec();
    input.extend_from_slice(b"bad");
    input.extend_from_slice(&(FIXTURE.len() as u32).to_be_bytes());
    input.extend_from_slice(FIXTURE.as_bytes());
    let latest = Latest::default();
    let wakes = AtomicUsize::new(0);
    assert!(receive(input.as_slice(), &latest, &|| {
        wakes.fetch_add(1, Ordering::Relaxed);
    })
    .await
    .is_err());
    assert!(latest.take().unwrap().is_ok());
    assert_eq!(wakes.load(Ordering::Relaxed), 1);
}

#[test]
fn t1795_native_gtk_changes_and_unregisters_on_private_display() {
    use blent_config::commands::SyncCommandExt;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("gtk.py");
    let fixture = root.path().join("native.py");
    std::fs::write(&source, include_str!("../gtk.py")).unwrap();
    std::fs::write(&fixture, include_str!("native_fixture.py")).unwrap();
    let output = std::process::Command::new("xvfb-run")
        .args(["-a", "/usr/bin/python3", "-I"])
        .arg(fixture)
        .arg(source)
        .env("GSETTINGS_BACKEND", "memory")
        .env_remove("GTK_THEME")
        .output_bounded()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}
