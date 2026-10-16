//! T1027: one source, shared original publication, and actual native retirement.
use super::*;
use blent_config::{
    host_activity::*,
    retirement::{Completion, Pool},
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
};

#[derive(Default)]
struct State {
    events: Mutex<VecDeque<Event>>,
    created: AtomicUsize,
    stopped: AtomicBool,
    lost: AtomicBool,
}
struct Factory {
    state: Arc<State>,
    capabilities: Capabilities,
    release: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
}
struct Input {
    state: Arc<State>,
    capabilities: Capabilities,
    native: Completion,
    _lifetime: SourceLifetime,
}
impl Source for Input {
    fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
    fn poll(&mut self) -> anyhow::Result<Option<Event>> {
        anyhow::ensure!(
            !self.state.lost.load(Ordering::Acquire),
            "T1027 source lost"
        );
        Ok(self.state.events.lock().unwrap().pop_front())
    }
    fn stop(&mut self) {
        self.state.stopped.store(true, Ordering::Release);
    }
    fn retired(&self) -> bool {
        self.native.is_complete()
    }
}
impl SourceFactory for Factory {
    fn create(&self, _: SourceSettings, lifetime: SourceLifetime) -> SourceCreation {
        self.state.created.fetch_add(1, Ordering::AcqRel);
        let release = self.release.lock().unwrap().take().unwrap();
        let slot = Pool::new(1).unwrap().reserve().unwrap();
        let native = slot.completion();
        lifetime.retain(native.clone());
        let thread = std::thread::spawn(move || {
            let _ = release.recv();
        });
        slot.retain(blent_config::retirement_work::Thread::new(thread));
        let source = Input {
            state: self.state.clone(),
            capabilities: self.capabilities,
            native,
            _lifetime: lifetime,
        };
        Box::pin(async move {
            Ok(StartedSource {
                epoch: 7,
                source: Box::new(source),
            })
        })
    }
}
struct Release(Option<std::sync::mpsc::Sender<()>>);
impl Drop for Release {
    fn drop(&mut self) {
        if let Some(release) = self.0.take() {
            let _ = release.send(());
        }
    }
}
fn fixture(capabilities: Capabilities) -> (Arc<Factory>, Release) {
    let (release, held) = std::sync::mpsc::channel();
    (
        Arc::new(Factory {
            state: Arc::new(State::default()),
            capabilities,
            release: Mutex::new(Some(held)),
        }),
        Release(Some(release)),
    )
}
fn running(revision: u64) -> Event {
    Event {
        epoch: 7,
        revision,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(false),
            display_on: Some(true),
        },
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
fn snapshot(view: &tokio::sync::watch::Receiver<Publication>) -> ActivitySnapshot {
    let Publication::Observed { snapshot, .. } = *view.borrow() else {
        panic!("T1027 observation missing")
    };
    snapshot
}
#[tokio::test]
async fn t1027_daemon_owns_one_source_and_preserves_preferences_edges_and_native_retirement() {
    let capabilities = Capabilities {
        power: true,
        lock: true,
        display: true,
    };
    let (factory, release) = fixture(capabilities);
    factory.state.events.lock().unwrap().push_back(running(1));
    let source = start(
        factory.clone(),
        Settings {
            pause_on_lock: false,
            ..Default::default()
        },
    )
    .unwrap();
    let views = [source.subscribe(), source.subscribe(), source.subscribe()];
    until(|| matches!(*views[0].borrow(), Publication::Observed { snapshot, .. } if snapshot.demand.stamp.revision == 1)).await;
    let original = snapshot(&views[0]).demand.stamp;
    factory.state.events.lock().unwrap().push_back(Event {
        state: HostState {
            locked: Some(true),
            ..running(2).state
        },
        ..running(2)
    });
    until(|| snapshot(&views[0]).demand.stamp.revision == 2).await;
    assert!(
        snapshot(&views[0]).demand.display,
        "T1027 saved lock preference lost"
    );
    factory.state.events.lock().unwrap().extend([
        Event {
            state: HostState {
                power: Some(Power::Suspending),
                ..running(3).state
            },
            ..running(3)
        },
        running(4),
    ]);
    until(|| snapshot(&views[0]).demand.stamp.revision == 4).await;
    for view in &views {
        assert!(view.same_channel(&views[0]));
        assert!(
            matches!(*view.borrow(), Publication::Observed { capabilities: seen, .. } if seen == capabilities)
        );
        assert!(snapshot(view).demand.display);
        assert!(snapshot(view).must_retire(ActivityTarget::Display, original));
        assert!(snapshot(view).must_retire(ActivityTarget::Auxiliary, original));
    }
    assert_eq!(factory.state.created.load(Ordering::Acquire), 1);
    let retiring = tokio::spawn(retire(source));
    until(|| factory.state.stopped.load(Ordering::Acquire)).await;
    assert!(
        !retiring.is_finished(),
        "T1027 original native source still held"
    );
    assert_eq!(*views[0].borrow(), Publication::Stopped);
    drop(release);
    tokio::time::timeout(Duration::from_secs(3), retiring)
        .await
        .unwrap()
        .unwrap();
}
#[tokio::test]
async fn t1027_unsupported_capabilities_and_loss_remain_visible_without_recreation() {
    for power in [false, true] {
        let capabilities = Capabilities {
            power,
            ..Default::default()
        };
        let (factory, release) = fixture(capabilities);
        factory.state.events.lock().unwrap().push_back(Event {
            state: HostState {
                power: power.then_some(Power::Running),
                ..Default::default()
            },
            ..running(1)
        });
        let source = start(factory.clone(), Default::default()).unwrap();
        let view = source.subscribe();
        until(|| matches!(*view.borrow(), Publication::Observed { .. })).await;
        assert!(
            matches!(*view.borrow(), Publication::Observed { capabilities: seen, .. } if seen == capabilities)
        );
        factory.state.lost.store(true, Ordering::Release);
        until(|| factory.state.stopped.load(Ordering::Acquire)).await;
        assert_eq!(snapshot(&view).demand.display, !power);
        assert_eq!(snapshot(&view).demand.auxiliary, !power);
        assert!(!source.retired());
        assert_eq!(factory.state.created.load(Ordering::Acquire), 1);
        drop(release);
        retire(source).await;
    }
}

#[test]
fn t1027_primary_and_adb_usb_extra_builders_forward_the_original_channel() {
    let (_, mut template, _stop) = crate::discovery_tests::monitor_inputs(3, (20000, 20001));
    let (published, view) = tokio::sync::watch::channel(Publication::Pending);
    template.activity = Some(view.clone());
    let primary = crate::session::Spec {
        capture: template.cap_template.clone(),
        ports: (20000, 20001),
        token: None,
        devices: (false, false, false),
    }
    .prepare(template.mode_tx.clone())
    .with_activity(Some(view.clone()));
    assert!(primary.activity.unwrap().same_channel(&view));
    // Both ADB extra startup and USB extra startup call this same constructor.
    for instance in 0..3 {
        let extra = crate::prepare_extra_session(&template, instance).unwrap();
        assert!(extra.activity.unwrap().same_channel(&view));
    }
    for instance in [3, 4, u32::MAX] {
        assert!(crate::prepare_extra_session(&template, instance).is_err());
    }
    published.send_replace(Publication::Failed);
    assert_eq!(
        *template.activity.as_ref().unwrap().borrow(),
        Publication::Failed
    );
    template.activity = None;
    assert!(crate::prepare_extra_session(&template, 1)
        .unwrap()
        .activity
        .is_none());
}

fn isolated_daemon() -> bool {
    const TEST: &str = "daemon_activity::tests::t1027_isolated_daemon_joins_original_source_and_handles_shutdown_requests";
    if std::env::var_os("BLENT_T1027_DAEMON").is_some() {
        return false;
    }
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700)).tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(&adb, "#!/bin/sh\nprintf 'List of devices attached\\n'\n").unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = std::process::Command::new("/usr/bin/unshare")
        .args([
            "--user",
            "--map-root-user",
            "--mount",
            "--pid",
            "--fork",
            "--mount-proc",
        ])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env("BLENT_T1027_DAEMON", "1")
        .env("HOME", root.path())
        .env("XDG_RUNTIME_DIR", root.path())
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        // Deliberately invalid broker state path: never start a native USB broker.
        .env("XDG_STATE_HOME", "t1027-no-native-broker")
        .env("PATH", root.path())
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/missing-t1027-bus")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("BLENT_FAKE_TABLET")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
async fn daemon_config() -> crate::Cli {
    use clap::Parser;
    let file = blent_config::FileConfig {
        encoder: "libx264".into(),
        encoding_gpu: "software".into(),
        input_touch: false,
        input_pen: false,
        input_pointer: false,
        check_updates: false,
        require_token: false,
        auto_launch_app: false,
        video_port: 29410,
        input_port: 29411,
        scheduling_priority: blent_config::scheduling::Priority::Normal,
        ..Default::default()
    };
    let path = crate::config::config_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, toml::to_string(&file).unwrap()).unwrap();
    crate::Cli::try_parse_from(["blent", "--helper", "/bin/true"]).unwrap()
}
async fn shutdown_requests() {
    for signal in [0, libc::SIGTERM, libc::SIGINT] {
        let (quit, receiver) = tokio::sync::watch::channel(false);
        let emit = tokio::spawn(async move {
            tokio::task::yield_now().await;
            if signal == 0 {
                quit.send_replace(true);
            } else {
                assert_eq!(unsafe { libc::kill(libc::getpid(), signal) }, 0);
            }
        });
        tokio::time::timeout(
            Duration::from_secs(3),
            crate::wait_daemon_shutdown(receiver),
        )
        .await
        .unwrap()
        .unwrap();
        emit.await.unwrap();
    }
}
#[tokio::test]
async fn t1027_isolated_daemon_joins_original_source_and_handles_shutdown_requests() {
    if isolated_daemon() {
        return;
    }
    shutdown_requests().await;
    let cli = daemon_config().await;
    let (factory, release) = fixture(Default::default());
    let observed = factory.clone();
    let mut daemon = tokio::spawn(crate::run_daemon_with(
        cli,
        factory.clone(),
        move |_| async move {
            until(|| observed.state.created.load(Ordering::Acquire) == 1).await;
            Ok(())
        },
    ));
    tokio::select! {
        _ = until(|| factory.state.stopped.load(Ordering::Acquire)) => {},
        result = &mut daemon => panic!("T1027 daemon returned before source retirement: {result:?}"),
    }
    assert!(
        !daemon.is_finished(),
        "T1027 daemon skipped original native source retirement"
    );
    drop(release);
    tokio::time::timeout(Duration::from_secs(5), daemon)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!crate::get_pid_path().exists());
    assert_eq!(factory.state.created.load(Ordering::Acquire), 1);
}

#[test]
fn t1045_extra_adb_usb_sessions_keep_saved_viewer_preferences() {
    let (_, mut template, _stop) = crate::discovery_tests::monitor_inputs(3, (20000, 20001));
    for pause_without_viewer in [false, true] {
        template.activity_preferences = Settings { pause_without_viewer, ..Default::default() };
        for instance in 0..3 {
            let extra = crate::prepare_extra_session(&template, instance).unwrap();
            assert_eq!(extra.activity_preferences, template.activity_preferences);
        }
    }
}
