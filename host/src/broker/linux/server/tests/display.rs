//! T908: actual TLS registration, exported session listeners and unchanged
//! video/WebSocket protocols, with synthetic capture/input and no device access.
use super::*;
use crate::{
    broker::{
        linux::{client::Launch, display_registration::Bound, presentation, registration_client},
        transport::{Event as WireEvent, Transport},
    },
    capture_contract::{CaptureBackend, CaptureContext, CaptureWorkers, Signal},
    connection_discovery::Discovery,
    display_connections::{Connections, Decision},
    input::{
        backend::{InputBackend, InputSink, PenSample},
        InputConfig,
    },
    registered_session::Bindings,
    session::{SessionResources, Spec},
    session_work::RuntimeWork,
};
use blent_config::connection::{
    management::{Credential, Decoder, Message as MediaMessage, Outcome, Profile},
    mux::Multiplexer,
    wire::Channel,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream},
    sync::{mpsc, oneshot, watch},
    task::JoinSet,
};
use tokio_tungstenite::{tungstenite::Message as WebSocketMessage, WebSocketStream};

struct Input(Arc<AtomicUsize>);
impl InputSink for Input {
    fn release_all(&self) {}
    fn touch(&self, position: (f64, f64, f64), action: u8, slot: u8) {
        assert_eq!(position, (0.25, 0.75, 0.5));
        assert_eq!((action, slot), (0, 0));
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn pen(&self, _: PenSample, _: bool) {
        panic!("T908 unexpected pen input")
    }
}
impl InputBackend for Input {
    fn sink(&self) -> Arc<dyn InputSink> {
        Arc::new(Self(self.0.clone()))
    }
    fn follow(
        &self,
        _: watch::Receiver<bool>,
        _: watch::Receiver<bool>,
        _: InputConfig,
    ) -> Signal<'static> {
        Box::pin(std::future::pending())
    }
}
struct Capture;
impl CaptureBackend for Capture {
    fn start(self: Box<Self>, context: CaptureContext) -> CaptureWorkers {
        CaptureWorkers {
            capture: Box::pin(frames(context)),
            auxiliary: vec![],
        }
    }
}
async fn frames(mut context: CaptureContext) -> Result<()> {
    let generation = Arc::new(AtomicBool::new(true));
    while !*context.stop.borrow() {
        if *context.display.borrow()
            && context.settings_rx.borrow().geometry_ready
            && context.video.receiver_count() > 0
        {
            let packet = crate::media::VideoPacket {
                data: crate::media_storage::MediaBytes::from_static(b"frame"),
                is_idr: true,
                seq: 7,
                codec_config: Some(crate::media_storage::MediaBytes::from_static(b"csd")),
                generation: generation.clone(),
            };
            let _ = context.video.send(packet);
        }
        tokio::select! {_=context.stop.changed()=>(),_=tokio::time::sleep(Duration::from_millis(10))=>()}
    }
    generation.store(false, Ordering::SeqCst);
    Ok(())
}
type Payload = (Channel, Vec<u8>);
type Clients = (DuplexStream, WebSocketStream<DuplexStream>);
struct Remote {
    wire: Transport<Box<dyn crate::accessory::Stream>>,
    decoder: Decoder,
    outputs: mpsc::Receiver<Payload>,
    sender: mpsc::Sender<Payload>,
    control: Option<mpsc::Sender<Vec<u8>>>,
    video: Option<mpsc::Sender<Vec<u8>>>,
    commit: Option<oneshot::Sender<()>>,
    clients: JoinSet<Clients>,
    pumps: JoinSet<()>,
}
impl Remote {
    fn new(stream: Box<dyn crate::accessory::Stream>) -> Self {
        let (sender, outputs) = mpsc::channel(8);
        Self {
            wire: Transport::new(
                stream,
                Multiplexer::new(1, 1, 4096, 64, 64).unwrap(),
                Duration::from_secs(2),
            )
            .unwrap(),
            decoder: Decoder::new(),
            outputs,
            sender,
            control: None,
            video: None,
            commit: None,
            clients: JoinSet::new(),
            pumps: JoinSet::new(),
        }
    }
    fn management(&mut self, message: MediaMessage) {
        let mut bytes = [0; blent_config::connection::management::MAX_RECORD];
        let count = message.encode(&mut bytes).unwrap();
        self.wire
            .multiplexer()
            .offer(Channel::Management, bytes[..count].to_vec())
            .unwrap();
    }
    fn command(&mut self, message: MediaMessage) {
        match message {
            MediaMessage::Offer(key, credential, Profile::Display) => {
                assert_eq!(key.resource, Resource::DisplaySession);
                let control = self.local(Channel::Control);
                let video = self.local(Channel::Display);
                let (commit, ready) = oneshot::channel();
                self.commit = Some(commit);
                self.clients.spawn(async move {
                    ready.await.unwrap();
                    client(video, control, credential).await
                });
                self.management(MediaMessage::Prepared(key));
            }
            MediaMessage::Commit(key) => {
                for channel in [Channel::Control, Channel::Display] {
                    self.wire
                        .multiplexer()
                        .activate(channel, key.generation)
                        .unwrap();
                }
                self.management(MediaMessage::Result(key, Outcome::Admitted));
                self.commit.take().unwrap().send(()).unwrap();
            }
            _ => panic!("T908 unexpected management command"),
        }
    }
    fn local(&mut self, channel: Channel) -> DuplexStream {
        let (stream, client) = tokio::io::duplex(4096);
        let (input, receiver) = mpsc::channel(8);
        match channel {
            Channel::Control => self.control = Some(input),
            Channel::Display => self.video = Some(input),
            _ => panic!("T908 invalid display channel"),
        }
        self.pumps
            .spawn(pump(stream, channel, self.sender.clone(), receiver));
        client
    }
    async fn received(&mut self, channel: Channel, length: u32) {
        let payload = self.wire.payload().to_vec();
        if channel == Channel::Management {
            let mut offset = 0;
            while offset < payload.len() {
                let (count, message) = self.decoder.feed(&payload[offset..]).unwrap();
                offset += count;
                if let Some(message) = message {
                    self.command(message);
                }
            }
        } else {
            let sender = match channel {
                Channel::Control => self.control.as_ref().unwrap(),
                Channel::Display => self.video.as_ref().unwrap(),
                _ => panic!("T908 unrelated channel"),
            };
            sender.send(payload).await.unwrap();
        }
        self.wire.multiplexer().release(channel, length).unwrap();
    }
    async fn run(mut self, mut stop: oneshot::Receiver<()>) -> Box<dyn crate::accessory::Stream> {
        loop {
            tokio::select! {
                _=&mut stop=>break,
                payload=self.outputs.recv()=>{let (channel,payload)=payload.unwrap();self.wire.multiplexer().offer(channel,payload).unwrap();},
                event=self.wire.next()=>if let WireEvent::Received{header,deliver}=event.unwrap() {if deliver {self.received(header.channel,header.length).await;}self.wire.consume().unwrap();},
            }
        }
        drop(self.clients.join_next().await.unwrap().unwrap());
        self.pumps.abort_all();
        while self.pumps.join_next().await.is_some() {}
        self.wire.into_stream()
    }
}
async fn pump(
    stream: DuplexStream,
    channel: Channel,
    outputs: mpsc::Sender<Payload>,
    mut input: mpsc::Receiver<Vec<u8>>,
) {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let mut bytes = [0; 512];
    loop {
        tokio::select! {
            read=reader.read(&mut bytes)=>{let count=read.unwrap();if count==0{break;}outputs.send((channel,bytes[..count].to_vec())).await.unwrap();},
            payload=input.recv()=>writer.write_all(&payload.unwrap()).await.unwrap(),
        }
    }
}
async fn json(client: &mut WebSocketStream<DuplexStream>, message: serde_json::Value) {
    client
        .send(WebSocketMessage::Text(message.to_string()))
        .await
        .unwrap();
}
async fn packet(video: &mut DuplexStream) -> (u8, Vec<u8>) {
    let length = video.read_u32().await.unwrap();
    assert!((1..=128).contains(&length));
    let mut bytes = vec![0; length as usize];
    video.read_exact(&mut bytes).await.unwrap();
    (bytes[0], bytes[1..].to_vec())
}
async fn client(mut video: DuplexStream, control: DuplexStream, credential: Credential) -> Clients {
    let token = credential
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    video.write_all(token.as_bytes()).await.unwrap();
    let (kind, payload) = packet(&mut video).await;
    assert_eq!((kind, payload), (0, b"csd".to_vec()));
    let (mut control, _) = tokio_tungstenite::client_async("ws://blent/", control)
        .await
        .unwrap();
    json(
        &mut control,
        serde_json::json!({"type":"auth","token":token}),
    )
    .await;
    assert!(matches!(
        control.next().await,
        Some(Ok(WebSocketMessage::Text(_)))
    ));
    json(&mut control,serde_json::json!({"type":"resolution","width":640,"height":480,"width_mm":310,"height_mm":194})).await;
    let (kind, payload) = packet(&mut video).await;
    assert_eq!(kind, 1);
    assert_eq!(&payload[..4], &7u32.to_be_bytes());
    assert_eq!(&payload[4..], b"frame");
    json(
        &mut control,
        serde_json::json!({"type":"rendered","seq":7,"decode_us":100}),
    )
    .await;
    json(
        &mut control,
        serde_json::json!({"type":"touch","x":0.25,"y":0.75,"pressure":0.5,"action":0,"slot":0}),
    )
    .await;
    (video, control)
}
fn launch(settings: &Settings) -> Launch {
    Launch {
        executable: "/bin/false".into(),
        runtime: settings.runtime.clone(),
        state: settings.state.clone(),
        helper: settings.helper.clone(),
    }
}
async fn admitted(
    connections: &mut Connections,
    slots: &mut crate::display_slots::Slots,
) -> crate::display_connections::Admission {
    loop {
        let now = std::time::Instant::now();
        connections.poll(slots, now).unwrap();
        if let Decision::Start(admission) = connections.plan(slots, now).unwrap() {
            return admission;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
#[tokio::test]
async fn t908_real_tls_exported_session_carries_config_frame_ack_and_touch_then_retires() {
    let auth = tls::authenticated().await;
    let launch = launch(&auth.fixture.server.settings);
    let (stop, done) = oneshot::channel();
    let running = tokio::spawn(async move {
        let mut fixture = auth.fixture;
        tokio::select! {result=fixture.server.run()=>panic!("server failed: {result:?}"),_=done=>()}
        fixture
    });
    let (remote_stop, remote_done) = oneshot::channel();
    let remote = tokio::spawn(Remote::new(auth.tablet).run(remote_done));
    let touches = Arc::new(AtomicUsize::new(0));
    let (shutdown, shutdown_rx) = watch::channel(false);
    let (mode, _) = watch::channel(false);
    let resources = SessionResources::default();
    resources
        .codec_config
        .publish(Some(crate::media_storage::MediaBytes::from_static(b"csd")));
    let prepared = Spec {
        settings: crate::display_test_support::settings(),
        instance: 0,
        ports: (0, 0),
        token: None,
        devices: (true, false, false),
    }
    .prepare(
        mode,
        resources,
        Box::new(Capture),
        Arc::new(Input(touches.clone())),
    );
    let settings = prepared.settings.clone();
    let (runtime, listeners) = prepared.start_exported(shutdown_rx).await.unwrap();
    let ports = listeners.ports();
    let source = Discovery::new(
        Box::new(presentation::Factory::new(presentation::Settings::new(launch.clone())).unwrap()),
        Duration::from_secs(1),
        std::time::Instant::now(),
    )
    .unwrap();
    let mut connections = Connections::new(source, vec![ports], Duration::from_secs(1)).unwrap();
    let mut slots = crate::display_slots::Slots::new(1).unwrap();
    let admission = bounded(admitted(&mut connections, &mut slots)).await;
    let factory = Arc::new(
        registration_client::Factory::new(registration_client::Settings::new(launch)).unwrap(),
    );
    connections.bind(
        &mut slots,
        admission,
        Bindings {
            activity: None,
            attachment: runtime.tablet_tx.clone(),
            registration: Box::new(Bound::new(factory, Arc::new(listeners))),
            entropy: blent_config::credentials::system_entropy,
            work: Box::new(RuntimeWork::owned(runtime)),
        },
    );
    bounded(async {
        while touches.load(Ordering::SeqCst) == 0 {
            connections
                .poll(&mut slots, std::time::Instant::now())
                .unwrap();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    assert!(settings.borrow().geometry_ready);
    let status = connections.status();
    assert_eq!(status.len(), 1);
    assert_eq!(status[0].identity.peer(), Some(&auth.pin));
    assert_eq!((status[0].video_port, status[0].input_port), ports);
    remote_stop.send(()).unwrap();
    let stream = bounded(remote).await.unwrap();
    drop(stream);
    connections.close(&mut slots);
    bounded(async {
        while !connections.retired() {
            connections
                .poll(&mut slots, std::time::Instant::now())
                .unwrap();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    assert!(connections.status().is_empty());
    for port in [ports.0, ports.1] {
        assert!(tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .is_ok());
    }
    stop.send(()).unwrap();
    let mut fixture = bounded(running).await.unwrap();
    assert!(fixture.server.routes.is_empty());
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
    drop(shutdown);
}
