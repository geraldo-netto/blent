use super::*;
use blent_config::{
    media_start::Start,
    retirement::{Fence, Pool},
    tablets::Identity,
};

fn successor() -> Result<Box<dyn AudioController>, String> {
    Ok(Box::new(Backend(
        Default::default(),
        RefCell::new(None),
        None,
    )))
}

fn prepared() -> (Panel, blent_config::retirement::Admission) {
    let pool = Pool::new(1).unwrap();
    let admission = pool.reserve().unwrap();
    let mut panel = Panel {
        backend: Some(Box::new(Backend(
            Default::default(),
            RefCell::new(None),
            Some(Fence::Pending(admission.completion())),
        ))),
        create: successor,
        ..Default::default()
    };
    panel
        .owner
        .admit(Some(Identity::Adb("first".into())), Start::acknowledged());
    (panel, admission)
}

fn options() -> AudioOptions {
    AudioOptions {
        serial: Some("second".into()),
        ..AudioOptions::new(Direction::Microphone)
    }
}

#[test]
fn t1549_retained_start_waits_for_exact_original_retirement() {
    let (mut panel, admission) = prepared();
    assert!(
        panel.start(options()).is_ok(),
        "T1549 discarded requested successor"
    );
    assert!(panel.backend.is_none());
    assert!(
        panel.start(options()).is_err(),
        "T1549 replaced already queued intent"
    );
    panel.status();
    assert_eq!(panel.target(), Some(&Identity::Adb("first".into())));
    assert!(panel.backend.is_none());
    drop(admission);
    panel.status();
    assert_eq!(panel.target(), Some(&Identity::Adb("second".into())));
    assert!(panel.backend.is_some());
    panel.status();
    panel.stop();
}

#[test]
fn t1549_stop_cancels_queued_start_without_retiring_original_early() {
    let (mut panel, admission) = prepared();
    panel.start(options()).unwrap();
    panel.stop();
    panel.status();
    assert_eq!(panel.target(), Some(&Identity::Adb("first".into())));
    drop(admission);
    panel.status();
    assert!(panel.target().is_none());
    assert!(panel.backend.is_none());
}

#[test]
fn t1549_successor_failure_is_reported_once_after_retirement() {
    let (mut panel, admission) = prepared();
    panel.create = || Err("successor unavailable".into());
    panel.start(options()).unwrap();
    drop(admission);
    panel.status();
    assert_eq!(panel.error(), Some("successor unavailable"));
    panel.create = || panic!("T1549 retried failed successor without user request");
    panel.status();
    assert!(panel.target().is_none());
    assert!(panel.backend.is_none());
}

#[test]
fn t1549_hidden_panel_keeps_progress_and_settles_without_another_click() {
    let (mut panel, admission) = prepared();
    let context = egui::Context::default();
    panel.maintain_start(&context);
    panel.start(options()).unwrap();
    panel.maintain_start(&context);
    assert!(panel.backend.is_none());
    drop(admission);
    panel.maintain_start(&context);
    assert_eq!(panel.target(), Some(&Identity::Adb("second".into())));
    panel.maintain_start(&context);
    panel.stop();
}

#[test]
fn t1549_immediate_retirement_preserves_successor_failure_feedback() {
    let (mut panel, admission) = prepared();
    drop(admission);
    panel.create = || Err("successor unavailable".into());
    panel.start_for(&Identity::Adb("second".into()), &options());
    panel.status();
    assert_eq!(panel.error(), Some("successor unavailable"));
}
