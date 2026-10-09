#[path = "camera_control.rs"]
mod camera_control;
#[path = "audio_control.rs"]
mod audio_control;

use blent_config::audio::{AudioController, AudioOptions, Direction};
use blent_config::camera::{CameraBackend, CameraProfile};
use std::{sync::mpsc, time::Duration};

fn held_drop<T: Send + 'static>(name: &str, owner: T, release: tokio::sync::watch::Sender<bool>) {
    let (done, finished) = mpsc::channel();
    let task = std::thread::spawn(move || { drop(owner); done.send(()).unwrap(); });
    let blocked = finished.recv_timeout(Duration::from_millis(200)).is_err();
    release.send_replace(true);
    finished.recv_timeout(Duration::from_secs(2)).unwrap();
    task.join().unwrap();
    assert!(blocked);
    println!("{name}_drop_waited_past_200ms={blocked}");
}

fn audio_drop() {
    let (release, wait) = tokio::sync::watch::channel(false);
    let (entered, ready) = mpsc::channel();
    let owner = audio_control::Controller::new(move |_, _, _| {
        let mut wait = wait.clone();
        let entered = entered.clone();
        async move {
            entered.send(()).unwrap();
            wait.wait_for(|released| *released).await.unwrap();
            Ok(())
        }
    }).unwrap();
    owner.start(AudioOptions::new(Direction::Microphone)).unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    held_drop("audio", owner, release);
}

fn camera_drop() {
    let (release, wait) = tokio::sync::watch::channel(false);
    let (entered, ready) = mpsc::channel();
    let owner = camera_control::Controller::new(move |_, _, _| {
        let mut wait = wait.clone();
        let entered = entered.clone();
        async move {
            entered.send(()).unwrap();
            wait.wait_for(|released| *released).await.unwrap();
            Ok(())
        }
    }).unwrap();
    owner.start(CameraProfile::default()).unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    held_drop("camera", owner, release);
}

struct Unwinding(mpsc::Sender<()>);
impl Drop for Unwinding { fn drop(&mut self) { let _ = self.0.send(()); } }
async fn injected_failure() -> anyhow::Result<()> { panic!("review-only injected worker panic"); }

fn worker_failure() {
    let (failed, signal) = mpsc::channel();
    let audio = audio_control::Controller::new(move |_, _, _| {
        let failed = failed.clone();
        async move { let _notice = Unwinding(failed); injected_failure().await }
    }).unwrap();
    audio.start(AudioOptions::new(Direction::Microphone)).unwrap();
    signal.recv_timeout(Duration::from_secs(2)).unwrap();
    println!("audio_after_worker_panic={:?}", audio.status().state);
    println!("audio_restart_error={}", audio.start(AudioOptions::new(Direction::Microphone)).unwrap_err());
    drop(audio);
    let (failed, signal) = mpsc::channel();
    let camera = camera_control::Controller::new(move |_, _, _| {
        let failed = failed.clone();
        async move { let _notice = Unwinding(failed); injected_failure().await }
    }).unwrap();
    camera.start(CameraProfile::default()).unwrap();
    signal.recv_timeout(Duration::from_secs(2)).unwrap();
    println!("camera_after_worker_panic={:?}", camera.state());
    drop(camera);
}

fn command_bytes() {
    use blent_config::commands::SyncCommandExt;
    let output = std::process::Command::new("python3")
        .args(["-c", "import os; b=b'x'*65536\nfor _ in range(64): os.write(1,b); os.write(2,b)"])
        .output_timeout(Duration::from_secs(2)).unwrap();
    assert!(output.status.success());
    println!("time_bounded_command_stdout_bytes={}", output.stdout.len());
    println!("time_bounded_command_stderr_bytes={}", output.stderr.len());
}

fn main() {
    audio_drop();
    camera_drop();
    worker_failure();
    command_bytes();
}
