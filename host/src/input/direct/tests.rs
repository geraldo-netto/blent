use super::*;
use blent_config::{
    direct_input::{Button, Contact, MouseAction},
    input_mapping::{Monitor, Rect, Rotation},
};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

#[derive(Default)]
struct Evidence {
    calls: Mutex<Vec<String>>,
    width: AtomicI32,
    fail: AtomicBool,
}
struct Fake(Arc<Evidence>);
struct Device(Arc<Evidence>);
impl Adapter for Device {
    fn touch(&mut self, frame: &[Contact]) -> Result<()> {
        self.0
            .calls
            .lock()
            .unwrap()
            .push(format!("touch {frame:?}"));
        ensure!(!self.0.fail.load(Ordering::SeqCst), "denied");
        Ok(())
    }
    fn mouse(&mut self, point: (u16, u16), action: MouseAction) -> Result<()> {
        self.0
            .calls
            .lock()
            .unwrap()
            .push(format!("mouse {point:?} {action:?}"));
        ensure!(!self.0.fail.load(Ordering::SeqCst), "denied");
        Ok(())
    }
    fn retire(&mut self) -> Result<()> {
        self.0.calls.lock().unwrap().push("retire".into());
        Ok(())
    }
}
impl Environment for Fake {
    type Device = Device;
    fn snapshot(&self) -> Result<Snapshot> {
        Snapshot::new(vec![Monitor {
            id: "screen".into(),
            name: "Screen".into(),
            bounds: Rect {
                left: 0,
                top: 0,
                right: 100 + self.0.width.load(Ordering::SeqCst),
                bottom: 100,
            },
            rotation: Rotation::Identity,
            scale_percent: 100,
            primary: true,
        }])
    }
    fn create(&self, mode: Mode) -> Result<Device> {
        self.0
            .calls
            .lock()
            .unwrap()
            .push(format!("create {mode:?}"));
        Ok(Device(self.0.clone()))
    }
    fn save(&self, config: &Config) -> Result<()> {
        self.0
            .calls
            .lock()
            .unwrap()
            .push(format!("save {:?}", config.mode));
        Ok(())
    }
}
fn backend(mode: Mode, touch: bool, mouse: bool) -> (Backend, Arc<Evidence>) {
    let evidence = Arc::new(Evidence::default());
    let factory = evidence.clone();
    (
        Backend::new(
            Config {
                monitor: "screen".into(),
                mode,
            },
            touch,
            mouse,
            move || Fake(factory),
        )
        .unwrap(),
        evidence,
    )
}
fn mouse(phase: Phase, button: Option<Button>, x: f64) -> Command {
    Command::Event {
        event: Event::Mouse {
            x,
            y: 0.5,
            phase,
            button,
        },
    }
}
async fn execute(sink: &dyn InputSink, command: Command) -> Result<()> {
    sink.direct(command)?.await
}

async fn wait_touches(evidence: &Evidence) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let count = evidence
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| call.starts_with("touch"))
                .count();
            if count == 3 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("T673 queued touch effects did not arrive");
}

async fn negotiate(sink: &dyn InputSink) {
    execute(sink, Command::Negotiate { version: 1 })
        .await
        .unwrap();
}

#[tokio::test]
async fn t673_tablet_and_mode_changes_retire_negotiation_before_reuse() {
    let (backend, _) = backend(Mode::DirectMouse, true, true);
    let sink = backend.sink();
    let (tablet, tablet_rx) = watch::channel(true);
    let (mode, mode_rx) = watch::channel(true);
    let task = tokio::spawn(backend.follow(tablet_rx, mode_rx, InputConfig::default()));
    negotiate(&sink).await;
    tablet.send(false).unwrap();
    wait_retired(&*sink).await;
    assert!(execute(&*sink, mouse(Phase::Move, None, 0.5))
        .await
        .is_err());
    tablet.send(true).unwrap();
    tokio::task::yield_now().await;
    negotiate(&sink).await;
    mode.send(false).unwrap();
    wait_retired(&*sink).await;
    negotiate(&sink).await;
    drop(mode);
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert!(!sink.direct_status().unwrap().negotiated);
}

async fn wait_retired(sink: &dyn InputSink) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while sink.direct_status().unwrap().negotiated {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t673_wire_dispatch_accepts_negotiated_touch_and_rejects_pen_and_failed_input() {
    let (backend, evidence) = backend(Mode::Touch, true, true);
    let sink = backend.sink();
    let (mode, _) = watch::channel(true);
    let settings_tx = None;
    let settings = super::super::SessionSettings::new(&settings_tx, &mode, false);
    let latency = crate::latency::LatencyTracker::new();
    let dispatch = |text: &str| {
        super::super::dispatch_event(
            serde_json::from_str(text).unwrap(),
            &*sink,
            &settings,
            &latency,
            false,
        )
    };
    assert!(
        dispatch(r#"{"type":"direct_input","command":{"type":"negotiate","version":1}}"#)
            .finish()
            .await
    );
    for action in [0, 2, 1] {
        assert!(
            dispatch(&format!(
                r#"{{"type":"touch","x":0.5,"y":0.5,"pressure":1,"action":{action},"slot":0}}"#
            ))
            .finish()
            .await
        );
    }
    assert!(
        !dispatch(
            r#"{"type":"pen","x":0.5,"y":0.5,"pressure":1,"tilt_x":0,"tilt_y":0,"action":0,"button":false}"#
        )
        .finish()
        .await
    );
    evidence.fail.store(true, Ordering::SeqCst);
    assert!(
        !dispatch(r#"{"type":"touch","x":0.5,"y":0.5,"pressure":1,"action":0,"slot":0}"#)
            .finish()
            .await
    );
    assert!(!sink.direct_status().unwrap().negotiated);
}

#[tokio::test]
async fn t673_negotiation_modes_bounds_and_failed_delivery_retire_owned_input() {
    let (backend, e) = backend(Mode::DirectMouse, true, true);
    let sink = backend.sink();
    assert!(!sink.direct_status().unwrap().negotiated);
    assert!(execute(&*sink, mouse(Phase::Move, None, 0.5))
        .await
        .is_err());
    assert!(e.calls.lock().unwrap().is_empty());
    for version in [0, 2, 255, u32::MAX] {
        assert!(execute(&*sink, Command::Negotiate { version })
            .await
            .is_err());
    }
    negotiate(&sink).await;
    execute(&*sink, mouse(Phase::Down, Some(Button::Left), 0.5))
        .await
        .unwrap();
    execute(&*sink, mouse(Phase::Move, None, 0.75))
        .await
        .unwrap();
    execute(&*sink, Command::Select { mode: Mode::Touch })
        .await
        .unwrap();
    let calls = e.calls.lock().unwrap().clone();
    assert!(
        calls.iter().position(|v| v == "retire").unwrap()
            < calls.iter().position(|v| v == "create Touch").unwrap()
    );
    execute(&*sink, touch_command(0.5, 0.5, 0, 0).unwrap())
        .await
        .unwrap();
    execute(&*sink, touch_command(0.75, 0.5, 2, 0).unwrap())
        .await
        .unwrap();
    execute(&*sink, touch_command(0.75, 0.5, 1, 0).unwrap())
        .await
        .unwrap();
    wait_touches(&e).await;
    assert!(e.calls.lock().unwrap().iter().any(|v| v.contains("touch")));
    assert!(touch_command(0.5, 0.5, 255, 255).is_err());
    sink.release_all();
    assert!(!sink.direct_status().unwrap().negotiated);
    for x in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        1.01,
        f64::MAX,
    ] {
        negotiate(&sink).await;
        execute(
            &*sink,
            Command::Select {
                mode: Mode::DirectMouse,
            },
        )
        .await
        .unwrap();
        assert!(execute(&*sink, mouse(Phase::Move, None, x)).await.is_err());
        assert!(!sink.direct_status().unwrap().negotiated);
    }
    negotiate(&sink).await;
    e.fail.store(true, Ordering::SeqCst);
    assert!(
        execute(&*sink, mouse(Phase::Down, Some(Button::Right), 0.0))
            .await
            .is_err()
    );
    e.fail.store(false, Ordering::SeqCst);
    negotiate(&sink).await;
    e.width.store(10, Ordering::SeqCst);
    assert!(execute(&*sink, mouse(Phase::Move, None, 1.0))
        .await
        .is_err());
    negotiate(&sink).await;
    execute(&*sink, mouse(Phase::Move, None, 1.0))
        .await
        .unwrap();
    sink.release_all();
    sink.release_all();
    assert!(execute(&*sink, Command::Select { mode: Mode::Touch })
        .await
        .is_err());
    sink.pen(
        PenSample {
            position: (0.0, 0.0, 0.0),
            tilt: (0.0, 0.0),
            eraser: false,
            action: 0,
            button: false,
        },
        true,
    );
}

#[tokio::test]
async fn t673_disabled_modes_and_seeded_wire_values_never_reach_devices() {
    for mode in [Mode::Touch, Mode::DirectMouse] {
        let (backend, e) = backend(mode, false, false);
        let sink = backend.sink();
        assert!(execute(&*sink, Command::Negotiate { version: 1 })
            .await
            .is_err());
        assert!(e.calls.lock().unwrap().is_empty());
    }
    for value in -32..=300 {
        let text =
            format!(r#"{{"type":"touch","x":0,"y":1,"pressure":1,"slot":{value},"action":0}}"#);
        let parsed = serde_json::from_str::<super::super::InputEvent>(&text);
        assert_eq!(parsed.is_ok(), (0..=255).contains(&value));
        if let Ok(super::super::InputEvent::Touch {
            x, y, action, slot, ..
        }) = parsed
        {
            let command = touch_command(x, y, action, slot).unwrap();
            let (backend, _) = backend(Mode::Touch, true, false);
            let sink = backend.sink();
            negotiate(&sink).await;
            assert_eq!(execute(&*sink, command).await.is_ok(), value < 10);
        }
    }
    for text in [
        r#"{"type":"negotiate","version":-1}"#,
        r#"{"type":"select","mode":"pen"}"#,
        r#"{"type":"event","event":{"type":"mouse","phase":"down","button":"extra","x":0,"y":0}}"#,
    ] {
        assert!(serde_json::from_str::<Command>(text).is_err());
    }
}

#[tokio::test]
async fn t673_authenticated_owner_replacement_and_disconnect_retire_buttons() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let (backend, e) = backend(Mode::DirectMouse, true, true);
    let proof = backend.completion();
    let (_tablet, tablet) = watch::channel(true);
    let (mode, _) = watch::channel(true);
    let server = super::super::InputServer::with_backend(
        InputConfig {
            port: 0,
            token: Some("secret".into()),
            touch: false,
            pen: false,
            pointer: false,
            ..Default::default()
        },
        None,
        mode,
        Default::default(),
        Arc::new(tokio::sync::Notify::new()),
        tablet,
        Arc::new(backend),
    );
    let listener = server.bind().await.unwrap();
    let address = format!("ws://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { server.run_with_listener(listener).await });
    let (mut denied, _) = tokio_tungstenite::connect_async(&address).await.unwrap();
    denied
        .send(Message::Text(r#"{"type":"auth","token":"wrong"}"#.into()))
        .await
        .unwrap();
    assert!(matches!(denied.next().await, Some(Ok(Message::Close(_)))));
    assert!(e.calls.lock().unwrap().is_empty());
    let (mut first, _) = tokio_tungstenite::connect_async(&address).await.unwrap();
    first
        .send(Message::Text(r#"{"type":"auth","token":"secret"}"#.into()))
        .await
        .unwrap();
    let greeting = first.next().await.unwrap().unwrap().into_text().unwrap();
    assert!(greeting.contains("direct_input"));
    first
        .send(Message::Text(
            r#"{"type":"direct_input","command":{"type":"negotiate","version":1}}"#.into(),
        ))
        .await
        .unwrap();
    assert!(first
        .next()
        .await
        .unwrap()
        .unwrap()
        .into_text()
        .unwrap()
        .contains("\"negotiated\":true"));
    first.send(Message::Text(serde_json::json!({"type":"direct_input","command":mouse(Phase::Down,Some(Button::Left),0.5)}).to_string().into())).await.unwrap();
    first.send(Message::Ping(vec![1].into())).await.unwrap();
    assert!(matches!(first.next().await, Some(Ok(Message::Pong(_)))));
    let (mut second, _) = tokio_tungstenite::connect_async(&address).await.unwrap();
    second
        .send(Message::Text(r#"{"type":"auth","token":"secret"}"#.into()))
        .await
        .unwrap();
    second.next().await.unwrap().unwrap();
    responsiveness::observed(&e, "retire", 1).await;
    second
        .send(Message::Text(
            r#"{"type":"direct_input","command":{"type":"negotiate","version":1}}"#.into(),
        ))
        .await
        .unwrap();
    second.next().await.unwrap().unwrap();
    second.close(None).await.unwrap();
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    task.abort();
    let _ = task.await;
    responsiveness::retired(proof).await;
    let calls = e.calls.lock().unwrap();
    assert!(calls.iter().filter(|call| *call == "retire").count() >= 2);
}

mod cleanup;
mod responsiveness;

#[tokio::test]
async fn t1503_non_command_touch_entry_revokes_direct_authority() {
    let (backend, evidence) = backend(Mode::Touch, true, false);
    let sink = backend.sink();
    negotiate(&sink).await;
    sink.touch((0.5, 0.5, 1.0), 0, 0);
    assert!(
        !sink.direct_status().unwrap().negotiated,
        "T1503 unreachable entry admitted touch"
    );
    wait_retired(&*sink).await;
    assert!(!evidence
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| call.contains("touch")));
}

#[tokio::test]
async fn t1499_top_level_touch_cancellation_reaches_the_native_contact() {
    let (backend, evidence) = backend(Mode::Touch, true, false);
    let sink = backend.sink();
    negotiate(&sink).await;
    execute(&*sink, touch_command(0.5, 0.5, 0, 0).unwrap())
        .await
        .unwrap();
    execute(&*sink, touch_command(0.0, 0.0, 3, 0).unwrap())
        .await
        .unwrap();
    let calls = evidence.calls.lock().unwrap().clone();
    assert!(
        calls.iter().any(|call| call.contains("Cancel")),
        "{calls:?}"
    );
    assert!(
        !calls.iter().any(|call| call.contains("phase: Up")),
        "{calls:?}"
    );
    sink.release_all();
    wait_retired(&*sink).await;
}

#[test]
fn t1499_touch_wire_accepts_only_four_defined_actions() {
    for action in 0..=u8::MAX {
        assert_eq!(touch_command(0.5, 0.5, action, 0).is_ok(), action <= 3);
    }
}
