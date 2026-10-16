use super::*;

struct StartupLease {
    entered: Option<oneshot::Sender<()>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

impl Drop for StartupLease {
    fn drop(&mut self) {
        self.entered.take().unwrap().send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
}

#[tokio::test]
async fn t1030_joined_startup_waits_for_original_dependent_cleanup() {
    let mut startup = Startup::launch(|| anyhow::bail!("T1030 startup error"), lifetime()).unwrap();
    let (entered, waiting) = oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    startup.task.as_mut().unwrap().keep_alive(StartupLease {
        entered: Some(entered),
        release: Mutex::new(held),
    });
    let waited_first = tokio::time::timeout(Duration::from_millis(10), startup.joined())
        .await
        .is_err();
    let (joined, mut done) = oneshot::channel();
    let (started, observing) = oneshot::channel();
    let observer = tokio::spawn(async move {
        started.send(()).unwrap();
        startup.joined().await;
        joined.send(()).unwrap();
    });
    tokio::time::timeout(Duration::from_secs(3), waiting)
        .await
        .unwrap()
        .unwrap();
    observing.await.unwrap();
    let released_early = done.try_recv().is_ok();
    release.send(()).unwrap();
    observer.await.unwrap();
    assert!(
        waited_first,
        "T1030 first startup join ignored dependent cleanup"
    );
    assert!(
        !released_early,
        "T1030 startup join released live dependent cleanup"
    );
}
use std::os::unix::fs::PermissionsExt;
fn program(root: &std::path::Path, source: &str) -> Program {
    let path = root.join("dpms-fixture");
    std::fs::write(&path, source).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    Program {
        executable: path,
        arguments: Vec::new(),
    }
}
async fn event(source: &mut LinuxSource) -> Event {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(event) = source.poll().unwrap() {
                return event;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap()
}
async fn retired(source: &LinuxSource) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !source.retired() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
struct OtherFields;
impl Backend for OtherFields {
    fn include(&self, caps: &mut Capabilities, state: &mut HostState) {
        caps.power = true;
        caps.lock = true;
        state.power = Some(blent_config::host_activity::Power::Running);
        state.locked = Some(false);
    }
    fn spawn(self: Box<Self>, _: Arc<Shared>) -> Result<Work> {
        Ok(Work::asynchronous(Task::spawn(async { Ok(()) })?))
    }
}
#[tokio::test]
async fn t1013_native_pipe_transitions_retain_original_process_until_actual_exit() {
    let root = tempfile::tempdir().unwrap();
    let producer = program(root.path(), "#!/bin/sh\nprintf '%s\\n' '{\"supported\":true,\"on\":true}'\nsleep .03\nprintf '%s\\n' '{\"supported\":true,\"on\":false}'\nsleep .03\nprintf '%s\\n' '{\"supported\":true,\"on\":true}'\nsleep 5\n");
    let probe = Probe::start(producer, Settings::default(), lifetime())
        .await
        .unwrap();
    let proofs = probe.startup.native.clone();
    let mut source = LinuxSource::start(
        fresh_epoch().unwrap(),
        Settings::default(),
        vec![Box::new(probe)],
        lifetime(),
    )
    .unwrap();
    assert!(source.capabilities().display);
    assert_eq!(event(&mut source).await.state.display_on, Some(true));
    assert_eq!(event(&mut source).await.state.display_on, Some(false));
    assert_eq!(event(&mut source).await.state.display_on, Some(true));
    source.stop();
    retired(&source).await;
    assert!(proofs.lock().unwrap().iter().all(Completion::is_complete));
}
#[tokio::test]
async fn t1013_native_pipe_extent_schema_and_timeout_reject_before_authority() {
    let root = tempfile::tempdir().unwrap();
    for body in [
        "printf '%s' '{\"supported\":true,\"on\":true}'",
        "printf '%s\\n' '{\"supported\":false,\"on\":true}'",
        "printf '%s\\n' 'invalid'",
        "sleep 5",
    ] {
        let producer = program(root.path(), &format!("#!/bin/sh\n{body}\n"));
        let settings = Settings {
            deadline: Duration::from_millis(50),
            ..Settings::default()
        };
        let probe = Probe::start(producer, settings, lifetime()).await.unwrap();
        assert!(!probe.initial.supported);
        let mut source = LinuxSource::start(
            fresh_epoch().unwrap(),
            settings,
            vec![Box::new(probe)],
            lifetime(),
        )
        .unwrap();
        source.stop();
        retired(&source).await;
    }
}
#[tokio::test]
async fn t1013_capability_withdrawal_or_stalled_original_clears_only_display_state() {
    let root = tempfile::tempdir().unwrap();
    for suffix in [
        "printf '%s\\n' '{\"supported\":false,\"on\":null}'",
        "sleep 5",
    ] {
        let producer = program(root.path(), &format!("#!/bin/sh\nprintf '%s\\n' '{{\"supported\":true,\"on\":true}}'\nsleep .03\n{suffix}\n"));
        let settings = Settings {
            deadline: Duration::from_millis(100),
            ..Settings::default()
        };
        let probe = Probe::start(producer, settings, lifetime()).await.unwrap();
        let mut source = LinuxSource::start(
            fresh_epoch().unwrap(),
            settings,
            vec![Box::new(OtherFields), Box::new(probe)],
            lifetime(),
        )
        .unwrap();
        event(&mut source).await;
        let lost = event(&mut source).await.state;
        assert_eq!(lost.display_on, None);
        assert_eq!(
            lost.power,
            Some(blent_config::host_activity::Power::Running)
        );
        assert_eq!(lost.locked, Some(false));
        retired(&source).await;
    }
}
#[tokio::test]
async fn t1013_unavailable_and_invalid_programs_retire_without_forged_capabilities() {
    assert!(Probe::start(
        Program {
            executable: "relative".into(),
            arguments: vec![]
        },
        Settings::default(),
        lifetime()
    )
    .await
    .is_err());
    let mut probe = Probe::start(
        Program {
            executable: "/__blent_missing_dpms__".into(),
            arguments: vec![],
        },
        Settings::default(),
        lifetime(),
    )
    .await
    .unwrap();
    assert!(!probe.initial.supported);
    probe.startup.joined().await;
    assert!(probe
        .startup
        .native
        .lock()
        .unwrap()
        .iter()
        .all(Completion::is_complete));
    assert!(Program::current(Settings::default())
        .unwrap()
        .executable
        .is_absolute());
}
#[test]
fn t1013_helper_heartbeat_fits_the_configured_observation_deadline() {
    let settings = Settings {
        deadline: Duration::from_millis(50),
        ..Settings::default()
    };
    let program = Program::current(settings).unwrap();
    let interval = Duration::from_nanos(program.arguments[1].to_str().unwrap().parse().unwrap());
    assert!(interval < Duration::from_millis(50));
    for deadline in [Duration::ZERO, Duration::from_secs(61)] {
        assert!(Program::current(Settings {
            deadline,
            ..settings
        })
        .is_err());
    }
}
#[tokio::test]
async fn t1013_timed_out_creator_keeps_native_proof_until_late_child_is_reaped() {
    let root = tempfile::tempdir().unwrap();
    let producer = program(root.path(), "#!/bin/sh\nsleep 5\n");
    let (release, hold) = std::sync::mpsc::channel();
    let (entered, start) = tokio::sync::oneshot::channel();
    let startup = Startup::launch(
        move || {
            entered.send(()).unwrap();
            hold.recv().unwrap();
            let mut command = Command::new(producer.executable);
            command.stdout(Stdio::piped()).stderr(Stdio::null());
            Ok(OwnedChild::spawn(&mut command)?)
        },
        lifetime(),
    )
    .unwrap();
    start.await.unwrap();
    let settings = Settings {
        deadline: Duration::from_millis(20),
        ..Settings::default()
    };
    let probe = Probe::from_startup(startup, settings).await.unwrap();
    let proofs = probe.startup.native.clone();
    let mut source = LinuxSource::start(
        fresh_epoch().unwrap(),
        settings,
        vec![Box::new(probe)],
        lifetime(),
    )
    .unwrap();
    source.stop();
    assert!(!source.retired());
    release.send(()).unwrap();
    retired(&source).await;
    assert_eq!(proofs.lock().unwrap().len(), 2);
    assert!(proofs.lock().unwrap().iter().all(Completion::is_complete));
}

#[tokio::test]
async fn t1018_cancelled_native_constructor_retains_scope_through_late_child_creation_and_reap() {
    let root = tempfile::tempdir().unwrap();
    let producer = program(root.path(), "#!/bin/sh\nsleep 5\n");
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let owner = SourceLifetime::new(pool.reserve().unwrap());
    let proof = owner.completion();
    let (release, hold) = std::sync::mpsc::channel();
    let (entered, start) = tokio::sync::oneshot::channel();
    let native = Arc::new(Mutex::new(None));
    let child_proof = native.clone();
    let startup = Startup::launch(
        move || {
            entered.send(()).unwrap();
            hold.recv().unwrap();
            let mut command = Command::new(producer.executable);
            command.stdout(Stdio::piped()).stderr(Stdio::null());
            let child = OwnedChild::spawn(&mut command)?;
            *child_proof.lock().unwrap() = Some(child.completion());
            Ok(child)
        },
        owner.clone(),
    )
    .unwrap();
    start.await.unwrap();
    assert!(tokio::time::timeout(
        Duration::from_millis(10),
        Probe::from_startup(startup, Settings::default())
    )
    .await
    .is_err());
    drop(owner);
    assert!(!proof.is_complete());
    assert!(pool.reserve().is_err());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !proof.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(native.lock().unwrap().as_ref().unwrap().is_complete());
    assert!(pool.reserve().is_ok());
}

fn lifetime() -> SourceLifetime {
    SourceLifetime::new(
        blent_config::retirement::Pool::new(1)
            .unwrap()
            .reserve()
            .unwrap(),
    )
}
