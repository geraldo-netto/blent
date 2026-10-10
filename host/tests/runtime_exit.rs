use blent_config::commands::SyncCommandExt;
use std::{process::Command, time::Duration};
#[test]
fn t819_runtime_exit_fixture() {
    if std::env::var_os("BLENT_T819_RUNTIME_FIXTURE").is_none() {
        return;
    }
    blent::runtime_owner::run(async {
        let (started, entered) = tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let _ = started.send(());
            // Native call never returns; only the disposable process can end it.
            loop {
                std::thread::park();
            }
        });
        entered.await.unwrap();
    })
    .unwrap();
}
#[test]
fn t819_final_runtime_exit_is_bounded_after_started_native_work() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "t819_runtime_exit_fixture", "--nocapture"])
        .env("BLENT_T819_RUNTIME_FIXTURE", "1")
        .env("BLENT_RUNTIME_STOP_MS", "20")
        .output_timeout(Duration::from_secs(2));
    assert!(
        output.is_ok(),
        "T819 final runtime destruction waited forever: {output:?}"
    );
    assert!(output.unwrap().status.success());
}
