//! T1189: the maintained native probe must compile against current decoder ownership.
#![cfg(target_os = "linux")]

#[tokio::test]
async fn t1189_current_camera_probe_compiles_with_native_retirement_contract() {
    use blent_config::commands::OwnedChild;
    use std::{process::Stdio, time::Duration};
    let mut command = tokio::process::Command::new(env!("CARGO"));
    command
        .args(["check", "--offline", "--locked", "--manifest-path"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .args(["--example", "camera-feedback-probe"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    let mut child = OwnedChild::spawn(&mut command).unwrap();
    let status = child.wait().await.unwrap();
    let retired = child.retire(Duration::ZERO).await;
    assert!(
        retired.completion.is_complete(),
        "T1189 compiler child not retired"
    );
    assert!(status.success(), "T1189 current probe failed to compile");
}

#[path = "../examples/camera-feedback-probe/ownership.rs"]
mod ownership;

#[tokio::test]
async fn t1189_no_start_proof_requires_the_unique_creator_to_close() {
    use futures_util::FutureExt;
    let (creation, retirement) = ownership::channel();
    let waiting = retirement.wait();
    tokio::pin!(waiting);
    assert!(waiting.as_mut().now_or_never().is_none());
    drop(creation);
    assert!(waiting.as_mut().now_or_never().is_some());
}

#[tokio::test]
async fn t1189_invalid_profile_seals_without_running_a_native_factory() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let called = Arc::new(AtomicUsize::new(0));
    let observed = called.clone();
    let (creation, retirement) = ownership::channel();
    let profile = blent_config::camera::CameraProfile {
        width: 0,
        ..Default::default()
    };
    let owner = creation.start(profile, move |_, _, _| async move {
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });
    assert!(owner.is_err());
    retirement.wait().await;
    assert_eq!(called.load(Ordering::SeqCst), 0);
}

struct Release([std::sync::mpsc::SyncSender<()>; 2]);
impl Release {
    fn advance(&self, stage: usize) {
        let _ = self.0[stage].try_send(());
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        self.advance(0);
        self.advance(1);
    }
}

#[tokio::test]
async fn t1189_cancelled_probe_keeps_original_creator_and_late_child_until_actual_retirement() {
    use blent::camera_control::{cancelled, NativeResource};
    use blent_config::commands::OwnedChild;
    use futures_util::FutureExt;
    use std::{process::Stdio, time::Duration};
    let (begin, creation_gate) = std::sync::mpsc::sync_channel(1);
    let (finish, retirement_gate) = std::sync::mpsc::sync_channel(1);
    let release = Release([begin, finish]);
    let (started, mut creator_ready) = tokio::sync::mpsc::channel(1);
    let (created, mut child_ready) = tokio::sync::mpsc::channel(1);
    let (creation, retirement) = ownership::channel();
    let owner = creation
        .start(Default::default(), move |_, mut stop, report| async move {
            let child_reservation = report.reserve_native(NativeResource::Decoder)?;
            let producer_reservation = report.reserve_native(NativeResource::Producer)?;
            let runtime = tokio::runtime::Handle::current();
            let task = blent::owned_task::Task::spawn_blocking(move || {
                let _ = started.try_send(());
                let _ = creation_gate.recv();
                let entered_runtime = runtime.enter();
                let mut command = tokio::process::Command::new("/bin/cat");
                command
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let child = OwnedChild::spawn(&mut command)?;
                let original = child.completion();
                child_reservation.bind(original.clone());
                let _ = created.try_send(original);
                let _ = retirement_gate.recv();
                drop(entered_runtime);
                runtime.block_on(child.retire(Duration::ZERO));
                Ok(())
            })?;
            producer_reservation.bind(task.completion());
            tokio::select! {
                result = task => result?,
                _ = cancelled(&mut stop) => Ok(()),
            }
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), creator_ready.recv())
        .await
        .unwrap()
        .unwrap();
    drop(owner);
    let waiting = retirement.wait();
    tokio::pin!(waiting);
    let creator_retained = waiting.as_mut().now_or_never().is_none();
    release.advance(0);
    let original_child = tokio::time::timeout(Duration::from_secs(5), child_ready.recv())
        .await
        .unwrap()
        .unwrap();
    let child_retained = !original_child.is_complete() && waiting.as_mut().now_or_never().is_none();
    release.advance(1);
    tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .unwrap();
    assert!(
        creator_retained,
        "T1189 cancelled probe released its original creator"
    );
    assert!(
        child_retained,
        "T1189 late native child escaped original retirement proof"
    );
    assert!(original_child.is_complete());
}
