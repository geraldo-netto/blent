use super::*;
use blent_config::camera::connection::{validate_credential, Invitation, InvitationResult};
use std::sync::Mutex;

struct IndependentInvitation(Mutex<Option<CameraProfile>>);
impl Invitation for IndependentInvitation {
    fn invite<'a>(
        &'a self,
        credential: &'a str,
        profile: &'a CameraProfile,
    ) -> InvitationResult<'a> {
        Box::pin(async move {
            validate_credential(credential)?;
            profile.validate()?;
            *self.0.lock().unwrap() = Some(profile.clone());
            Ok(())
        })
    }
}

#[tokio::test]
async fn t931_camera_session_invites_through_independent_port() {
    let profile = tests::options();
    let invitation = IndependentInvitation(Mutex::new(None));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token = "a".repeat(64);
    let session = Session {
        listener: &listener,
        invitation: &invitation,
        token: &token,
        ffmpeg: std::path::Path::new("/unused"),
        options: &profile,
    };
    let (stop, mut stopped) = watch::channel(false);
    let (state, observed) = watch::channel(State::Starting);
    let report = Report {
        state,
        preview: watch::channel(None).0,
    };
    let frames = [watch::channel(None).0, watch::channel(None).0];
    let mut producers = Producers::new();
    // Keep the producer branch pending until the session has observed stop.
    let task = crate::owned_task::Task::spawn(std::future::pending()).unwrap();
    producers.push(task);
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        let cancel = async {
            while invitation.0.lock().unwrap().is_none() {
                tokio::task::yield_now().await;
            }
            stop.send_replace(true);
        };
        tokio::join!(
            session.invited(&frames, &mut producers, &mut stopped, &report),
            cancel
        )
        .0
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(*observed.borrow(), State::Waiting);
    assert_eq!(*invitation.0.lock().unwrap(), Some(profile.profile));
    producers.iter().next().unwrap().abort();
    assert!(producers.next().await.unwrap().is_err());
}

#[cfg(not(feature = "accessory-transport"))]
#[tokio::test]
async fn t931_native_camera_selection_fails_before_output_device_ownership() {
    let mut options = tests::options();
    let root = tempfile::tempdir().unwrap();
    options.front_device = root.path().join("missing-front");
    options.rear_device = root.path().join("missing-rear");
    options.usb_peer = Some("a".repeat(64));
    let (_stop, stopped) = watch::channel(false);
    let report = Report {
        state: watch::channel(State::Starting).0,
        preview: watch::channel(None).0,
    };
    let error = run_native(options, stopped, report).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "direct-USB camera requires accessory-transport support"
    );
}

#[tokio::test]
async fn t931_adb_adapter_rejects_native_target_without_a_command() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("command-started");
    let bridge = bridge::Bridge {
        adb: tests::script(
            root.path(),
            "adb",
            &format!("import pathlib\npathlib.Path({marker:?}).touch()"),
        ),
        serial: "tablet".into(),
        remote: "tcp:1234".into(),
        local: "tcp:5678".into(),
    };
    let mut profile = CameraProfile::default();
    profile.usb_peer = Some("a".repeat(64));
    let port: &dyn Invitation = &bridge;
    let error = port.invite(&"a".repeat(64), &profile).await.unwrap_err();
    assert_eq!(error.to_string(), "ADB camera invitation cannot target USB");
    assert!(!marker.exists());
}

struct PendingInvitation;
impl Invitation for PendingInvitation {
    fn invite<'a>(&'a self, _: &'a str, _: &'a CameraProfile) -> InvitationResult<'a> {
        Box::pin(std::future::pending())
    }
}

#[tokio::test(start_paused = true)]
async fn t971_stop_cancels_pending_invitation_before_output_retirement() {
    let options = tests::options();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token = "a".repeat(64);
    let session = Session {
        listener: &listener,
        invitation: &PendingInvitation,
        token: &token,
        ffmpeg: std::path::Path::new("/unused"),
        options: &options,
    };
    let (stop, mut stopped) = watch::channel(false);
    let report = Report {
        state: watch::channel(State::Starting).0,
        preview: watch::channel(None).0,
    };
    let frames = [watch::channel(None).0, watch::channel(None).0];
    let mut producers = Producers::new();
    let producer = crate::owned_task::Task::spawn(std::future::pending()).unwrap();
    producers.push(producer);
    let cancel = async {
        tokio::task::yield_now().await;
        stop.send_replace(true);
    };
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        tokio::join!(
            session.invited(&frames, &mut producers, &mut stopped, &report),
            cancel
        )
        .0
    })
    .await;
    assert!(
        result.is_ok(),
        "T971 Stop must interrupt pending invitation"
    );
    result.unwrap().unwrap();
    // Cancellation of invitation must not claim the independent producer exited.
    assert!(!producers.is_empty());
    producers.iter().next().unwrap().abort();
    assert!(producers.next().await.unwrap().is_err());
}

#[tokio::test]
async fn t971_producer_failure_interrupts_pending_invitation() {
    let options = tests::options();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let session = Session {
        listener: &listener,
        invitation: &PendingInvitation,
        token: &"a".repeat(64),
        ffmpeg: std::path::Path::new("/unused"),
        options: &options,
    };
    let (_stop, mut stopped) = watch::channel(false);
    let report = Report {
        state: watch::channel(State::Starting).0,
        preview: watch::channel(None).0,
    };
    let frames = [watch::channel(None).0, watch::channel(None).0];
    for failure in [false, true] {
        let mut producers = Producers::new();
        producers.push(
            crate::owned_task::Task::spawn(async move {
                if failure {
                    anyhow::bail!("T971 original producer failed");
                }
                Ok(())
            })
            .unwrap(),
        );
        let error = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            session.invited(&frames, &mut producers, &mut stopped, &report),
        )
        .await
        .unwrap()
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            if failure {
                "T971 original producer failed"
            } else {
                "camera producer stopped"
            }
        );
    }
    assert_eq!(
        producer_stopped(&mut Producers::new())
            .await
            .unwrap_err()
            .to_string(),
        "camera producers stopped"
    );
}

impl Connection for IndependentInvitation {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(std::future::pending())
    }
    fn close(&self) -> InvitationResult<'_> {
        Box::pin(async { Ok(()) })
    }
}
impl Connection for PendingInvitation {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(std::future::pending())
    }
    fn close(&self) -> InvitationResult<'_> {
        Box::pin(async { Ok(()) })
    }
}

struct LostConnection(IndependentInvitation);
impl Invitation for LostConnection {
    fn invite<'a>(&'a self, token: &'a str, profile: &'a CameraProfile) -> InvitationResult<'a> {
        self.0.invite(token, profile)
    }
}
impl Connection for LostConnection {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(async { anyhow::bail!("T933 original connection lost") })
    }
    fn close(&self) -> InvitationResult<'_> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn t933_connection_loss_stops_sessions_without_claiming_producer_retirement() {
    let options = tests::options();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let connection = LostConnection(IndependentInvitation(Mutex::new(None)));
    let session = Session {
        listener: &listener,
        invitation: &connection,
        token: &"a".repeat(64),
        ffmpeg: std::path::Path::new("/unused"),
        options: &options,
    };
    let (_stop, mut stopped) = watch::channel(false);
    let report = Report {
        state: watch::channel(State::Starting).0,
        preview: watch::channel(None).0,
    };
    let frames = [watch::channel(None).0, watch::channel(None).0];
    let mut producers = Producers::new();
    producers.push(crate::owned_task::Task::spawn(std::future::pending()).unwrap());
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        session.invited(&frames, &mut producers, &mut stopped, &report),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(error.to_string(), "T933 original connection lost");
    assert_eq!(*connection.0 .0.lock().unwrap(), Some(options.profile));
    assert!(!producers.is_empty());
    producers.iter().next().unwrap().abort();
    assert!(producers.next().await.unwrap().is_err());
}

#[cfg(feature = "accessory-transport")]
#[tokio::test]
async fn t933_usb_selection_reaches_output_ownership_without_adb() {
    let root = tempfile::tempdir().unwrap();
    let options = CameraOptions {
        profile: CameraProfile {
            usb_peer: Some("a".repeat(64)),
            ..Default::default()
        },
        front_device: root.path().join("missing-front"),
        rear_device: root.path().join("missing-rear"),
        ..Default::default()
    };
    let (_stop, stopped) = watch::channel(false);
    let report = Report {
        state: watch::channel(State::Starting).0,
        preview: watch::channel(None).0,
    };
    let expected = format!("Open {}:", options.front_device.display());
    let error = run_native(options, stopped, report).await.unwrap_err();
    assert!(error.to_string().starts_with(&expected), "{error:#}");
}

#[test]
fn t933_connection_factory_uses_private_commands_and_explicit_transport() {
    if std::env::var_os("BLENT_T933_FACTORY_CHILD").is_some() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(factory_fixture());
        return;
    }
    let root = tempfile::tempdir().unwrap();
    tests::script(root.path(), "adb", "import sys\nif 'tcp:0' in sys.argv: print('34567')\nelif '--list' in sys.argv: print('Usb tcp:34567 tcp:1')\nelse: print('tablet')");
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "camera::connection_tests::t933_connection_factory_uses_private_commands_and_explicit_transport", "--nocapture"])
        .env("BLENT_T933_FACTORY_CHILD", "1").env("PATH", root.path())
        .env("XDG_RUNTIME_DIR", root.path().join("runtime"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

async fn factory_fixture() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token = "a".repeat(64);
    let adb = connection(&CameraProfile::default(), &listener, &token)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(1), adb.monitor())
            .await
            .is_err()
    );
    adb.close().await.unwrap();
    let profile = CameraProfile {
        usb_peer: Some("b".repeat(64)),
        ..Default::default()
    };
    assert!(
        validate_transport(profile.target().unwrap()).is_ok()
            == cfg!(feature = "accessory-transport")
    );
    let usb = connection(&profile, &listener, &token).await;
    #[cfg(feature = "accessory-transport")]
    usb.unwrap().close().await.unwrap();
    #[cfg(not(feature = "accessory-transport"))]
    assert!(usb.is_err());
    let invalid = CameraProfile {
        serial: Some("adb".into()),
        ..profile
    };
    assert!(connection(&invalid, &listener, &token).await.is_err());
}
