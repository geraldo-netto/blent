//! Original authenticated private sockets observe the existing machine source.
use super::super::*;
use crate::capture_contract::{CaptureBackend, CaptureContext, CaptureWorkers};
use blent_config::host_activity::*;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

type Client =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
#[derive(Default)]
struct Backend(AtomicUsize);
impl InputSink for Backend {
    fn release_all(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn touch(&self, _: (f64, f64, f64), _: u8, _: u8) {
        panic!("T1056 no physical input")
    }
    fn pen(&self, _: PenSample, _: bool) {
        panic!("T1056 no physical pen input")
    }
}
impl InputBackend for Backend {
    fn sink(&self) -> Arc<dyn InputSink> {
        Arc::new(Self::default())
    }
    fn follow(
        &self,
        _: watch::Receiver<bool>,
        _: watch::Receiver<bool>,
        _: InputConfig,
    ) -> crate::capture_contract::Signal<'static> {
        Box::pin(std::future::pending())
    }
}
struct Capture;
impl CaptureBackend for Capture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        CaptureWorkers {
            auxiliary: vec![],
            capture: Box::pin(async move {
                let _ = context.stop.wait_for(|stop| *stop).await;
                Ok(())
            }),
        }
    }
}
struct Fixture {
    runtime: crate::session::Runtime,
    _listeners: crate::session_network::RetainedListeners,
    _shutdown: watch::Sender<bool>,
}
impl Fixture {
    async fn new(source: Option<watch::Receiver<Publication>>) -> Self {
        let (mode, _) = watch::channel(false);
        let prepared = crate::session::Spec {
            settings: crate::display_test_support::settings(),
            instance: 0,
            ports: (0, 0),
            token: Some("ab".repeat(32)),
            devices: (false, false, false),
        }
        .prepare(
            mode,
            Default::default(),
            Box::new(Capture),
            Arc::new(Backend::default()),
        )
        .with_activity(source);
        let (shutdown, stopped) = watch::channel(false);
        let (runtime, listeners) = prepared.start_exported(stopped).await.unwrap();
        Self {
            runtime,
            _listeners: listeners,
            _shutdown: shutdown,
        }
    }
    async fn connect(&self, token: &str) -> Client {
        let (mut client, _) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}", self.runtime.input_port))
                .await
                .unwrap();
        client
            .send(Message::Text(
                serde_json::json!({"type":"auth","token":token}).to_string(),
            ))
            .await
            .unwrap();
        client
    }
    async fn stop(self) {
        let done = self.runtime.stop().await;
        tokio::time::timeout(Duration::from_secs(3), async {
            while !done.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
}
async fn value(client: &mut Client) -> serde_json::Value {
    let message = tokio::time::timeout(Duration::from_secs(3), client.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_str(message.to_text().unwrap()).unwrap()
}
fn signal(value: &serde_json::Value) -> blent_config::display_activity::Signal {
    serde_json::from_value(value["display_activity"].clone()).unwrap()
}
fn update(feed: &mut Feed, revision: u64, locked: bool) -> Publication {
    feed.observe(Event {
        epoch: 1,
        revision,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(locked),
            display_on: Some(true),
        },
    })
    .unwrap();
    Publication::Observed {
        capabilities: Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        snapshot: feed.snapshot(true, false),
    }
}
#[tokio::test]
async fn t1056_portable_session_binds_original_source_before_initial_authenticated_greeting() {
    let (published, source) = watch::channel(Publication::Pending);
    let fixture = Fixture::new(Some(source)).await;
    let mut client = fixture.connect(&"ab".repeat(32)).await;
    let greeting = value(&mut client).await;
    assert_eq!(greeting["status"], "connected");
    assert_eq!(signal(&greeting).revision(), 1);
    assert!(!signal(&greeting).active());
    let mut feed = Feed::new(
        1,
        Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        Settings::default(),
    )
    .unwrap();
    published.send_replace(update(&mut feed, 1, false));
    assert!(signal(&value(&mut client).await).active());
    published.send_replace(update(&mut feed, 2, true));
    published.send_replace(update(&mut feed, 3, false));
    let resumed = signal(&value(&mut client).await);
    assert!(resumed.active());
    assert_eq!(resumed.stopped(), 1);
    drop(published);
    assert!(!signal(&value(&mut client).await).active());
    client.close(None).await.unwrap();
    fixture.stop().await;
}
#[tokio::test]
async fn t1056_unsupported_source_omits_hint_and_wrong_auth_never_receives_one() {
    let fixture = Fixture::new(None).await;
    let mut client = fixture.connect(&"ab".repeat(32)).await;
    assert!(value(&mut client).await.get("display_activity").is_none());
    client.close(None).await.unwrap();
    let mut rejected = fixture.connect(&"cd".repeat(32)).await;
    let first = tokio::time::timeout(Duration::from_secs(3), rejected.next())
        .await
        .unwrap();
    assert!(!matches!(first, Some(Ok(Message::Text(_)))));
    fixture.stop().await;
}
#[tokio::test]
async fn t1056_replaced_controller_cannot_publish_after_current_owner_claims_socket() {
    let (published, source) = watch::channel(Publication::Pending);
    let fixture = Fixture::new(Some(source)).await;
    let mut original = fixture.connect(&"ab".repeat(32)).await;
    value(&mut original).await;
    let mut replacement = fixture.connect(&"ab".repeat(32)).await;
    assert_eq!(signal(&value(&mut replacement).await).revision(), 1);
    published.send_replace(Publication::Failed);
    let old = tokio::time::timeout(Duration::from_secs(3), original.next())
        .await
        .unwrap();
    assert!(!matches!(old, Some(Ok(Message::Text(_)))));
    replacement.close(None).await.unwrap();
    fixture.stop().await;
}

#[tokio::test]
async fn t1056_authentication_and_original_attachment_cutoff_fence_every_hint() {
    let mut feed = Feed::new(
        1,
        Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        Settings::default(),
    )
    .unwrap();
    let (published, source) = watch::channel(update(&mut feed, 1, false));
    let fixture = Fixture::new(Some(source)).await;
    let mut rejected = fixture.connect(&"cd".repeat(32)).await;
    let unauthorized = tokio::time::timeout(Duration::from_secs(3), rejected.next())
        .await
        .unwrap();
    assert!(!matches!(unauthorized, Some(Ok(Message::Text(_)))));
    let mut client = fixture.connect(&"ab".repeat(32)).await;
    assert!(signal(&value(&mut client).await).active());
    fixture.runtime.tablet_tx.retire();
    published.send_replace(update(&mut feed, 2, true));
    let retired = tokio::time::timeout(Duration::from_secs(3), client.next())
        .await
        .unwrap();
    assert!(!matches!(retired, Some(Ok(Message::Text(_)))));
    fixture.stop().await;
}
