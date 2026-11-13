//! Review-only direct restore exit probe; exclusively uses the fail-closed fixture.
#[path = "../../../../../host/tests/input_commands/fixture.rs"]
mod fixture;
use std::{path::PathBuf, time::Duration};

async fn abandon(started: std::sync::mpsc::Sender<()>) {
    let task = tokio::spawn(blent::osk::restore());
    let child = PathBuf::from(std::env::var_os("BLENT_T1433_CHILD").unwrap());
    while std::fs::read_to_string(&child)
        .unwrap_or_default()
        .is_empty()
    {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    started.send(()).unwrap();
}

async fn deadline(started: std::sync::mpsc::Sender<()>) {
    let release = PathBuf::from(std::env::var_os("BLENT_T1433_RELEASE").unwrap());
    let child = PathBuf::from(std::env::var_os("BLENT_T1433_CHILD").unwrap());
    std::fs::write(&release, "warm probe").unwrap();
    assert!(matches!(
        blent::kwin::backend().await,
        Some(blent::kwin::Backend::Busctl)
    ));
    std::fs::remove_file(child).unwrap();
    std::fs::remove_file(release).unwrap();
    started.send(()).unwrap();
    blent::osk::restore().await;
}

fn observe(cancel: bool) {
    let release = fixture::Release::new();
    let (started, request) = std::sync::mpsc::channel();
    let (finished, returned) = std::sync::mpsc::channel();
    let owner = std::thread::spawn(move || {
        if cancel {
            blent::runtime_owner::run(abandon(started)).unwrap();
        } else {
            blent::runtime_owner::run(deadline(started)).unwrap();
        }
        finished.send(()).unwrap();
    });
    request.recv().unwrap();
    let early = returned.recv_timeout(Duration::from_secs(7)).is_ok();
    let pid = std::fs::read_to_string(std::env::var_os("BLENT_T1433_CHILD").unwrap()).unwrap();
    let child = PathBuf::from(format!("/proc/{pid}"));
    let alive = child.exists();
    drop(release);
    owner.join().unwrap();
    while child.exists() {
        std::thread::sleep(Duration::from_millis(1));
    }
    let report = format!("cancel={cancel} runtime_returned_before_release={early} original_child_alive={alive} calls={}\n", fixture::calls());
    println!("{report}");
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::var_os("BLENT_REVIEW_RESULT").unwrap())
        .unwrap()
        .write_all(report.as_bytes())
        .unwrap();
    assert!(early && alive, "candidate not reproduced");
}

#[test]
fn direct_restore_exit() {
    if std::env::var_os("BLENT_T1433_CHILD").is_none() {
        fixture::run("direct_restore_exit", "64");
        return;
    }
    fixture::intercepted();
    observe(true);
}

#[test]
fn direct_restore_deadline_exit() {
    if std::env::var_os("BLENT_T1433_CHILD").is_none() {
        fixture::run("direct_restore_deadline_exit", "64");
        return;
    }
    fixture::intercepted();
    observe(false);
}
