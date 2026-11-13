#![cfg(target_os = "linux")]
//! T1441: direct restoration retains its original command through process exit.
#[path = "input_commands/fixture.rs"]
mod fixture;
use std::{path::PathBuf, sync::mpsc, time::Duration};

fn prepared_restore() -> impl std::future::Future<Output = ()> + Send {
    blent::osk::Restoration::new().unwrap().run()
}
fn child_path() -> PathBuf {
    std::env::var_os("BLENT_T1433_CHILD").unwrap().into()
}
fn release_path() -> PathBuf {
    std::env::var_os("BLENT_T1433_RELEASE").unwrap().into()
}
async fn original_started() {
    while std::fs::read_to_string(child_path())
        .unwrap_or_default()
        .is_empty()
    {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn warm_backend() {
    std::fs::write(release_path(), "warm probe").unwrap();
    assert!(matches!(
        blent::kwin::backend().await,
        Some(blent::kwin::Backend::Busctl)
    ));
    std::fs::remove_file(child_path()).unwrap();
    std::fs::remove_file(release_path()).unwrap();
}
async fn cancelled(started: mpsc::Sender<()>) {
    warm_backend().await;
    let operation = tokio::spawn(prepared_restore());
    original_started().await;
    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    started.send(()).unwrap();
}
async fn deadline(started: mpsc::Sender<()>, cutoff: bool) {
    warm_backend().await;
    let restore = prepared_restore();
    if cutoff {
        blent::runtime_owner::request_shutdown();
    }
    started.send(()).unwrap();
    restore.await;
}

struct Owner {
    release: Option<fixture::Release>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Owner {
    fn settle(&mut self) {
        drop(self.release.take());
        self.thread.take().unwrap().join().unwrap();
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        drop(self.release.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn pending_exit(mode: &str) {
    let (started, entered) = mpsc::channel();
    let (finished, returned) = mpsc::channel();
    let mode = mode.to_owned();
    let thread = std::thread::spawn(move || {
        blent::runtime_owner::run(async {
            match mode.as_str() {
                "cancel" => cancelled(started).await,
                _ => deadline(started, mode == "cutoff").await,
            }
        })
        .unwrap();
        finished.send(()).unwrap();
    });
    let mut owner = Owner {
        release: Some(fixture::Release::new()),
        thread: Some(thread),
    };
    entered.recv().unwrap();
    // Observe beyond the five-second command deadline without killing or truncating work.
    let early = returned.recv_timeout(Duration::from_secs(7)).is_ok();
    let pid = std::fs::read_to_string(child_path()).unwrap();
    let child = PathBuf::from(format!("/proc/{pid}"));
    let original_alive = child.exists();
    owner.settle();
    while child.exists() {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        original_alive,
        "fixture lost its original command before release"
    );
    assert!(
        !early,
        "T1441 process entry returned before original OSK restoration settled"
    );
}

fn successful_restore() {
    let unused = blent::osk::Restoration::new().unwrap();
    assert_eq!(
        blent::osk::Restoration::new().err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(unused);
    let _release = fixture::Release::new();
    std::fs::write(release_path(), "normal completion").unwrap();
    blent::runtime_owner::run(prepared_restore()).unwrap();
    assert_eq!(
        blent::osk::Restoration::new().err().unwrap().kind(),
        std::io::ErrorKind::BrokenPipe
    );
    assert!(fixture::calls() >= 2);
    let backup =
        PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/share/blent/osk-restore");
    assert!(
        !backup.exists(),
        "successful restoration retained obsolete backup"
    );
}

#[test]
fn t1441_direct_restore_fixture() {
    let Ok(mode) = std::env::var("BLENT_T1441_MODE") else {
        return;
    };
    fixture::intercepted();
    if mode == "normal" {
        successful_restore();
    } else {
        pending_exit(&mode);
    }
}

#[test]
fn t1441_direct_restore_exit_retains_cancelled_and_timed_out_originals() {
    if std::env::var_os("BLENT_T1433_CHILD").is_some() {
        t1441_direct_restore_fixture();
        return;
    }
    for mode in ["cancel", "deadline", "cutoff", "normal"] {
        // Each fixture child owns all environment changes and its original command.
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "t1441_private_launcher", "--nocapture"])
            .env("BLENT_T1441_MODE", mode)
            .env("BLENT_RUNTIME_STOP_MS", "20")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{mode}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
#[test]
fn t1441_private_launcher() {
    if std::env::var_os("BLENT_T1441_MODE").is_some() {
        fixture::run(
            "t1441_direct_restore_exit_retains_cancelled_and_timed_out_originals",
            "64",
        );
    }
}
