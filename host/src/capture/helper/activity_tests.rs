//! T1035: actual helper endpoints remain bound to original native ownership.
use super::*;
use crate::capture::ownership_tests::isolated;
use std::{
    collections::HashSet,
    sync::{mpsc, Mutex},
    time::Duration,
};

pub(in crate::capture) fn configuration() -> CaptureConfig {
    use std::os::unix::fs::PermissionsExt;
    let root = crate::runtime::runtime_dir().unwrap();
    let path = root.join("t1035-helper");
    std::fs::write(
        &path,
        r#"#!/usr/bin/python3
import os, pathlib, socket, sys, time
root = pathlib.Path(__file__).parent
mode = os.environ.get('BLENT_T1035_MODE', 'echo')
args = sys.argv[1:]
control = None
if '--capture-activity-fd' in args:
    fd = int(args[args.index('--capture-activity-fd') + 1])
    control = socket.socket(fileno=fd)
    control.setblocking(True)
    (root / 'inherited').write_text(str(fd))
if mode != 'silent':
    print('EVDI_CONNECTED card' + ('-1' if mode == 'bad' else '4294967295'), flush=True)
if control is None:
    while True: time.sleep(0.01)
while True:
    request = control.recv(32)
    if not request: break
    with (root / 'requests').open('a') as output: output.write(request.hex() + '\n')
    while (root / 'hold').exists() and not (root / 'release').exists(): time.sleep(0.001)
    if mode == 'foreign': request = request[:8] + bytes([1 - request[8]])
    control.send(request)
"#,
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    CaptureConfig {
        helper_path: path,
        edid_path: Some(root.join("unused.edid")),
        encoder: "libx264".into(),
        raw_transport: blent_config::raw_frame::RawTransport::Fifo,
        ..Default::default()
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
fn requests() -> Vec<String> {
    std::fs::read_to_string(crate::runtime::runtime_dir().unwrap().join("requests"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}
async fn retire(helper: &mut HelperProcess) {
    helper.terminate().await;
    until(|| helper.pending.as_ref().unwrap().is_complete()).await;
    until(|| {
        helper
            .stdout_pending
            .as_ref()
            .is_none_or(|proof| proof.is_complete())
    })
    .await;
}
#[tokio::test]
async fn t1035_legacy_helper_stays_uncontrolled_and_unavailable_is_explicit() {
    if isolated("capture::helper::activity_tests::t1035_legacy_helper_stays_uncontrolled_and_unavailable_is_explicit") { return; }
    let config = configuration();
    let mut helper = HelperProcess::new();
    assert!(helper.set_capture_active(false).await.is_err());
    helper.start(&config).await.unwrap();
    assert!(!crate::runtime::runtime_dir()
        .unwrap()
        .join("inherited")
        .exists());
    assert!(helper.set_capture_active(false).await.is_err());
    retire(&mut helper).await;
}
#[tokio::test]
async fn t1035_original_helper_acknowledges_without_monitor_or_child_replacement() {
    if isolated("capture::helper::activity_tests::t1035_original_helper_acknowledges_without_monitor_or_child_replacement") { return; }
    let config = configuration();
    let mut helper = HelperProcess::new();
    helper.enable_activity();
    helper.start(&config).await.unwrap();
    let pid = helper.child.as_ref().unwrap().id();
    let inherited: i32 =
        std::fs::read_to_string(crate::runtime::runtime_dir().unwrap().join("inherited"))
            .expect("T1035 helper did not inherit activity endpoint")
            .parse()
            .unwrap();
    assert!(inherited >= 3);
    for (index, active) in [true, false, true, false].into_iter().enumerate() {
        let request = helper.set_capture_active(active).await.unwrap();
        assert_eq!(request.sequence(), index as u64 + 1);
        assert_eq!(request.active(), active);
        assert_eq!(helper.child.as_ref().unwrap().id(), pid);
        assert_eq!(helper.card, Some(u32::MAX));
    }
    assert_eq!(requests().len(), 4);
    retire(&mut helper).await;
    assert!(helper.set_capture_active(true).await.is_err());
}
#[tokio::test]
async fn t1035_cancelled_transition_must_finish_original_ack_before_resume() {
    if isolated("capture::helper::activity_tests::t1035_cancelled_transition_must_finish_original_ack_before_resume") { return; }
    let config = configuration();
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("hold"), "hold").unwrap();
    let mut helper = HelperProcess::new();
    helper.enable_activity();
    helper.start(&config).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(30), helper.set_capture_active(false))
            .await
            .is_err(),
        "T1035 pause claimed completion before original ack"
    );
    until(|| requests().len() == 1).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(30), helper.set_capture_active(true))
            .await
            .is_err()
    );
    assert_eq!(requests(), ["000000000000000100"]);
    std::fs::write(root.join("release"), "release").unwrap();
    let resumed = helper.set_capture_active(true).await.unwrap();
    assert_eq!(resumed.sequence(), 2);
    assert!(resumed.active());
    assert_eq!(helper.set_capture_active(true).await.unwrap(), resumed);
    assert_eq!(requests(), ["000000000000000100", "000000000000000201"]);
    retire(&mut helper).await;
    helper.start(&config).await.unwrap();
    assert_eq!(helper.set_capture_active(true).await.unwrap().sequence(), 1);
    retire(&mut helper).await;
}
#[tokio::test]
async fn t1035_foreign_ack_poison_cannot_grant_native_transition() {
    if isolated(
        "capture::helper::activity_tests::t1035_foreign_ack_poison_cannot_grant_native_transition",
    ) {
        return;
    }
    let config = configuration();
    let mut helper = HelperProcess::new();
    helper.enable_activity();
    helper
        .start_with(&config, |command| {
            Child::spawn(command.env("BLENT_T1035_MODE", "foreign"))
        })
        .await
        .unwrap();
    assert!(helper
        .set_capture_active(false)
        .await
        .unwrap_err()
        .to_string()
        .contains("foreign capture acknowledgment"));
    assert!(helper.set_capture_active(true).await.is_err());
    assert!(helper.is_running());
    retire(&mut helper).await;
}
struct Held(mpsc::Sender<()>, Mutex<mpsc::Receiver<()>>);
impl Drop for Held {
    fn drop(&mut self) {
        let _ = self.0.send(());
        let _ = self.1.lock().unwrap().recv();
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
fn sockets() -> HashSet<std::path::PathBuf> {
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|entry| std::fs::read_link(entry.ok()?.path()).ok())
        .filter(|path| path.to_string_lossy().starts_with("socket:["))
        .collect()
}
async fn held_startup(mode: &str) {
    let config = configuration();
    let before = sockets();
    let mut helper = HelperProcess::new();
    helper.enable_activity();
    let (entered, ready) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        helper.start_with(&config, |command| {
            let mut child = Child::spawn(command.env("BLENT_T1035_MODE", mode))?;
            child.keep_alive(Held(entered, Mutex::new(held)));
            Ok(child)
        }),
    )
    .await;
    assert!(!matches!(result, Ok(Ok(()))));
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    let original = helper.pending.as_ref().unwrap().clone();
    let endpoints: HashSet<_> = sockets().difference(&before).cloned().collect();
    assert!(
        !endpoints.is_empty(),
        "T1035 original control endpoint absent"
    );
    helper.stop();
    assert!(
        endpoints.is_subset(&sockets()),
        "T1035 child cleanup lost original socket lease"
    );
    assert!(!original.is_complete());
    assert!(helper
        .start_with(&config, |_| panic!(
            "T1035 replacement acquired before original cleanup"
        ))
        .await
        .is_err());
    drop(release);
    until(|| original.is_complete()).await;
    assert!(endpoints.is_disjoint(&sockets()));
    helper.start(&config).await.unwrap();
    assert_eq!(helper.set_capture_active(true).await.unwrap().sequence(), 1);
    retire(&mut helper).await;
}
#[tokio::test]
async fn t1035_cancelled_startup_retains_original_socket_until_native_cleanup() {
    if isolated("capture::helper::activity_tests::t1035_cancelled_startup_retains_original_socket_until_native_cleanup") { return; }
    held_startup("silent").await;
}
#[tokio::test]
async fn t1035_failed_handshake_retains_original_socket_until_native_cleanup() {
    if isolated("capture::helper::activity_tests::t1035_failed_handshake_retains_original_socket_until_native_cleanup") { return; }
    held_startup("bad").await;
}
#[tokio::test]
async fn t1035_spawn_failure_releases_unexposed_endpoints_and_allows_fresh_retry() {
    if isolated("capture::helper::activity_tests::t1035_spawn_failure_releases_unexposed_endpoints_and_allows_fresh_retry") { return; }
    let config = configuration();
    let mut helper = HelperProcess::new();
    helper.enable_activity();
    let before = sockets();
    assert!(helper
        .start_with(&config, |_| Err(std::io::Error::other(
            "T1035 injected spawn failure"
        )))
        .await
        .is_err());
    helper.stop();
    assert_eq!(sockets(), before);
    helper.start(&config).await.unwrap();
    assert_eq!(helper.set_capture_active(true).await.unwrap().sequence(), 1);
    retire(&mut helper).await;
}

#[cfg(feature = "inproc-encoder")]
#[tokio::test]
async fn t1035_shared_raw_and_activity_endpoints_remain_independent() {
    if isolated("capture::helper::activity_tests::t1035_shared_raw_and_activity_endpoints_remain_independent") { return; }
    let mut config = configuration();
    config.raw_transport = blent_config::raw_frame::RawTransport::SharedMemory;
    let mut helper = HelperProcess::new();
    helper.enable_activity();
    helper.start(&config).await.unwrap();
    assert!(helper.raw_socket.is_some());
    assert!(helper.set_capture_active(true).await.unwrap().active());
    assert!(!helper.set_capture_active(false).await.unwrap().active());
    retire(&mut helper).await;
    assert!(
        helper.raw_socket.is_some(),
        "T1035 activity retirement invalidated independent raw endpoint"
    );
}
