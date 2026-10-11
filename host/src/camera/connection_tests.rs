use super::*;
use blent_config::camera::connection::{validate_credential, InvitationResult};
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
        "direct-USB camera sharing is not yet supported"
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
