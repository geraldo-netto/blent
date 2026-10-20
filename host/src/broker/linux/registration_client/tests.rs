use super::*;
use crate::broker::linux::{
    async_io::{AsyncListener, AsyncSocket},
    owner::Owner,
};
use blent_config::connection::{
    management::{Credential, Profile, Resource},
    pairing::PeerId,
    registration::{Message, Rejection, State as RouteState},
    routes::RouteId,
};
use std::os::unix::fs::PermissionsExt;

pub(super) fn request() -> Request {
    Request {
        peer: PeerId::parse("ab".repeat(32)).unwrap(),
        resource: Resource::DisplaySession,
        credential: Credential::new([9; 32]),
        profile: Profile::Display,
    }
}
pub(super) fn listeners() -> Vec<TcpListener> {
    vec![
        TcpListener::bind("127.0.0.1:0").unwrap(),
        TcpListener::bind("127.0.0.1:0").unwrap(),
    ]
}
fn route() -> RouteId {
    RouteId {
        owner: 7,
        revision: 4,
    }
}
async fn bounded<T>(work: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), work)
        .await
        .unwrap()
}
async fn until(mut condition: impl FnMut() -> bool) {
    bounded(async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
}
struct Fixture {
    _root: tempfile::TempDir,
    listener: AsyncListener,
    factory: Factory,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let listener = AsyncListener::new(Owner::bind(root.path(), 8).unwrap()).unwrap();
        let settings = Settings::new(Launch {
            executable: "/bin/false".into(),
            runtime: root.path().into(),
            state: root.path().into(),
            helper: root.path().join("unused"),
        });
        Self {
            _root: root,
            listener,
            factory: Factory::new(settings).unwrap(),
        }
    }
    async fn connected(&self) -> (Handle, AsyncSocket) {
        let client = self.factory.start(request(), listeners()).unwrap();
        let (server, _) = bounded(self.listener.accept()).await.unwrap();
        let packet = bounded(server.receive()).await.unwrap();
        assert!(matches!(
            packet.message,
            Message::Register { request: 1, .. }
        ));
        assert_eq!(packet.descriptors.len(), 2);
        (client, server)
    }
    async fn active(&self) -> (Handle, AsyncSocket) {
        let (client, server) = self.connected().await;
        server
            .send(
                &Message::Registered {
                    request: 1,
                    route: route(),
                },
                &[],
            )
            .await
            .unwrap();
        status(&server, 1, RouteState::Active).await;
        until(|| client.status().phase == Phase::Active).await;
        (client, server)
    }
}
async fn status(server: &AsyncSocket, request: u64, state: RouteState) {
    bounded(server.send(
        &Message::Status {
            request,
            route: route(),
            state,
        },
        &[],
    ))
    .await
    .unwrap();
}
#[tokio::test]
async fn t911_close_while_idle_sends_unregister_without_waiting_for_another_reply() {
    let fixture = Fixture::new();
    let (client, server) = fixture.active().await;
    client.close();
    assert_eq!(client.status().phase, Phase::Closing);
    assert!(!client.retired());
    let packet = bounded(server.receive()).await.unwrap();
    assert!(
        packet.message
            == Message::Unregister {
                request: 2,
                route: route()
            }
    );
    assert!(packet.descriptors.is_empty());
    status(&server, 2, RouteState::Retired).await;
    until(|| client.retired()).await;
    assert!(client.status().remote_retired);
}

#[tokio::test]
async fn t911_cancel_during_registration_preserves_descriptors_until_exact_retirement() {
    let mut fixture = Fixture::new();
    fixture.factory.pool = Pool::new(1).unwrap();
    let endpoints = listeners();
    let addresses = endpoints
        .iter()
        .map(|v| v.local_addr().unwrap())
        .collect::<Vec<_>>();
    let client = fixture.factory.start(request(), endpoints).unwrap();
    let (server, _) = bounded(fixture.listener.accept()).await.unwrap();
    let packet = bounded(server.receive()).await.unwrap();
    drop(packet);
    client.close();
    assert!(!client.retired());
    assert!(fixture.factory.start(request(), listeners()).is_err());
    assert!(addresses
        .iter()
        .all(|address| TcpListener::bind(address).is_err()));
    server
        .send(
            &Message::Registered {
                request: 1,
                route: route(),
            },
            &[],
        )
        .await
        .unwrap();
    let packet = bounded(server.receive()).await.unwrap();
    assert!(matches!(
        packet.message,
        Message::Unregister { request: 2, .. }
    ));
    status(&server, 1, RouteState::Active).await;
    assert_ne!(client.status().phase, Phase::Active);
    status(&server, 2, RouteState::Retired).await;
    until(|| client.retired()).await;
    assert!(addresses
        .iter()
        .all(|address| TcpListener::bind(address).is_ok()));
}
#[tokio::test]
async fn t911_rejected_registration_releases_without_claiming_active() {
    let fixture = Fixture::new();
    let (client, server) = fixture.connected().await;
    server
        .send(
            &Message::Rejected {
                request: 1,
                reason: Rejection::Unavailable,
            },
            &[],
        )
        .await
        .unwrap();
    until(|| client.retired()).await;
    assert_eq!(client.status().phase, Phase::Failed);
    assert_eq!(
        client.status().failure,
        Some(Failure::Rejected(Rejection::Unavailable))
    );
    assert!(client.status().remote_retired);
}
#[tokio::test]
async fn t911_foreign_reply_revokes_and_keeps_cleanup_channel_for_exact_proof() {
    let fixture = Fixture::new();
    let (client, server) = fixture.active().await;
    server
        .send(
            &Message::Status {
                request: 1,
                route: RouteId {
                    owner: 8,
                    revision: 4,
                },
                state: RouteState::Retired,
            },
            &[],
        )
        .await
        .unwrap();
    let packet = bounded(server.receive()).await.unwrap();
    assert!(matches!(
        packet.message,
        Message::Unregister { request: 2, .. }
    ));
    assert_eq!(client.status().failure, Some(Failure::Protocol));
    assert!(!client.retired());
    status(&server, 2, RouteState::Retired).await;
    until(|| client.retired()).await;
}
#[tokio::test]
async fn t911_lost_ipc_quarantines_capacity_and_listener_ownership() {
    let mut fixture = Fixture::new();
    fixture.factory.pool = Pool::new(2).unwrap();
    let (client, server) = fixture.active().await;
    server.close();
    until(|| client.status().failure.is_some()).await;
    assert!(!client.retired());
    assert!(!client.status().remote_retired);
    let (sibling, remote) = fixture.connected().await;
    remote
        .send(
            &Message::Rejected {
                request: 1,
                reason: Rejection::Busy,
            },
            &[],
        )
        .await
        .unwrap();
    until(|| sibling.retired()).await;
    assert_eq!(fixture.factory.pool.active(), 1);
    assert!(!client.retired());
    // White-box proof injection cleans the isolated fixture only. Production
    // never synthesizes retirement when its IPC lifetime has been lost.
    client
        .shared
        .update(|model| {
            model.receive(Message::Status {
                request: 1,
                route: route(),
                state: RouteState::Retired,
            })
        })
        .unwrap();
    until(|| client.retired()).await;
}
#[tokio::test]
async fn t911_timeout_and_repeated_close_cannot_release_uncertain_registration() {
    let mut fixture = Fixture::new();
    fixture.factory.settings.cleanup = Duration::from_millis(60);
    let (client, server) = fixture.active().await;
    client.close();
    bounded(server.receive()).await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while client.status().failure.is_none() {
        assert!(Instant::now() < deadline);
        client.close();
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert_eq!(client.status().failure, Some(Failure::TimedOut));
    assert!(!client.retired());
    client
        .shared
        .update(|model| {
            model.receive(Message::Status {
                request: 2,
                route: route(),
                state: RouteState::Retired,
            })
        })
        .unwrap();
    until(|| client.retired()).await;
}

#[tokio::test]
async fn t1228_admission_timeout_sends_unregister_and_receives_original_retirement() {
    let mut fixture = Fixture::new();
    fixture.factory.settings.admission = Duration::from_millis(40);
    let (client, server) = fixture.connected().await;
    server
        .send(
            &Message::Registered {
                request: 1,
                route: route(),
            },
            &[],
        )
        .await
        .unwrap();
    status(&server, 1, RouteState::Queued).await;
    until(|| client.status().failure == Some(Failure::TimedOut)).await;
    assert!(
        !client.retired(),
        "T1228 timeout cannot release original endpoints"
    );
    let packet = bounded(server.receive()).await.unwrap();
    assert!(
        packet.message
            == Message::Unregister {
                request: 2,
                route: route()
            }
    );
    assert!(packet.descriptors.is_empty());
    assert!(!client.status().remote_retired);
    status(&server, 2, RouteState::Retired).await;
    until(|| client.retired()).await;
    assert_eq!(client.status().failure, Some(Failure::TimedOut));
    assert!(client.status().remote_retired);
}

#[tokio::test]
async fn t1234_consent_default_matches_the_shared_resource_policy() {
    let settings = Fixture::new().factory.settings;
    assert_eq!(settings.admission, Duration::from_secs(120));
    let shared = blent_config::connection::resources::AdmissionTimeout::default();
    assert_eq!(
        settings.admission.as_millis(),
        shared.milliseconds() as u128
    );
    assert_eq!(settings.startup, Duration::from_secs(10));
    assert_eq!(settings.cleanup, Duration::from_secs(5));
}

#[tokio::test]
async fn t1234_consent_accepts_the_resource_domain_and_rejects_invalid_boundaries() {
    let fixture = Fixture::new();
    for milliseconds in [1, 37, 61_000, 120_000, 600_000] {
        let mut settings = fixture.factory.settings.clone();
        settings.admission = Duration::from_millis(milliseconds);
        assert!(Factory::new(settings).is_ok(), "T1234 {milliseconds}ms");
    }
    for duration in [
        Duration::ZERO,
        Duration::from_nanos(999_999),
        Duration::from_secs(600) + Duration::from_nanos(1),
        Duration::MAX,
    ] {
        let mut settings = fixture.factory.settings.clone();
        settings.admission = duration;
        assert!(Factory::new(settings).is_err(), "T1234 {duration:?}");
    }
}

#[tokio::test]
async fn t911_drop_sends_cancellation_and_retains_owner_until_broker_proof() {
    let fixture = Fixture::new();
    let (client, server) = fixture.active().await;
    let completion = client.completion.clone();
    drop(client);
    let packet = bounded(server.receive()).await.unwrap();
    assert!(matches!(
        packet.message,
        Message::Unregister { request: 2, .. }
    ));
    assert!(!completion.is_complete());
    status(&server, 2, RouteState::Retired).await;
    until(|| completion.is_complete()).await;
}
#[tokio::test]
async fn t911_missing_broker_cancels_before_exposure_and_releases_after_native_exit() {
    let mut fixture = Fixture::new();
    fixture.factory.settings.startup = Duration::from_millis(30);
    fixture.factory.connector = Arc::new(
        Connector::new(
            Launch {
                executable: "/bin/false".into(),
                runtime: fixture._root.path().join("absent"),
                state: fixture._root.path().into(),
                helper: "/bin/false".into(),
            },
            1,
        )
        .unwrap(),
    );
    let client = fixture.factory.start(request(), listeners()).unwrap();
    until(|| client.retired()).await;
    assert_eq!(client.status().failure, Some(Failure::Native));
    assert!(client.status().remote_retired);
    let canceled = fixture.factory.start(request(), listeners()).unwrap();
    canceled.close();
    until(|| canceled.retired()).await;
    assert!(canceled.status().remote_retired);
}
#[tokio::test]
async fn t911_settings_and_endpoint_inputs_reject_invalid_boundaries() {
    let fixture = Fixture::new();
    for capacity in [0, 257, usize::MAX] {
        let mut settings = fixture.factory.settings.clone();
        settings.capacity = capacity;
        assert!(Factory::new(settings).is_err());
    }
    for duration in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        for field in 0..2 {
            let mut settings = fixture.factory.settings.clone();
            match field {
                0 => settings.startup = duration,
                _ => settings.cleanup = duration,
            }
            assert!(Factory::new(settings).is_err());
        }
    }
    for count in [0, 1, 3] {
        let listeners = (0..count)
            .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
            .collect();
        assert!(fixture.factory.start(request(), listeners).is_err());
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    assert!(fixture
        .factory
        .start(request(), vec![listener.try_clone().unwrap(), listener])
        .is_err());
    let mut invalid = request();
    invalid.resource = Resource::Camera;
    assert!(fixture.factory.start(invalid, listeners()).is_err());
    assert_eq!(fixture.factory.pool.active(), 0);
}

#[tokio::test]
async fn t911_unexpected_descriptor_packet_cannot_replace_the_registered_route() {
    use std::os::fd::AsFd;
    let fixture = Fixture::new();
    let (client, server) = fixture.active().await;
    let endpoints = listeners();
    server
        .send(
            &Message::Register {
                request: 1,
                peer: request().peer,
                resource: Resource::DisplaySession,
                credential: request().credential,
                profile: Profile::Display,
            },
            &[endpoints[0].as_fd(), endpoints[1].as_fd()],
        )
        .await
        .unwrap();
    let cancel = bounded(server.receive()).await.unwrap();
    assert!(matches!(
        cancel.message,
        Message::Unregister { request: 2, .. }
    ));
    assert_eq!(client.status().route, Some(route()));
    assert_eq!(client.status().failure, Some(Failure::Protocol));
    assert!(!client.retired());
    status(&server, 2, RouteState::Retired).await;
    until(|| client.retired()).await;
}
#[tokio::test]
async fn t911_remote_retirement_alone_does_not_release_a_blocked_native_owner() {
    let shared = Arc::new(Shared::new(Model::new(request()).unwrap()));
    shared.update(Model::close);
    let endpoints = Arc::new(
        Endpoints::validate(
            Resource::DisplaySession,
            listeners().into_iter().map(OwnedFd::from).collect(),
        )
        .unwrap(),
    );
    let pool = Pool::new(1).unwrap();
    let admission = pool.reserve().unwrap();
    let completion = admission.completion();
    let (release, wait) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || wait.recv_timeout(Duration::from_secs(3)).unwrap());
    admission.retain(Retained {
        thread: retirement_work::Thread::new(thread),
        shared: shared.clone(),
        _endpoints: endpoints,
    });
    assert!(shared.state().model.status().remote_retired);
    assert!(!completion.is_complete());
    assert!(pool.reserve().is_err());
    release.send(()).unwrap();
    until(|| completion.is_complete()).await;
    assert_eq!(pool.active(), 0);
}

// T1007: real separate process keeps received endpoints after closing IPC. The
// parent proves socket loss alone cannot release them, then observes actual exit.
#[tokio::test]
async fn t1007_broker_process_fixture() {
    use std::io::BufRead;
    let Some(directory) = std::env::var_os("BLENT_T1007_BROKER_FIXTURE") else {
        return;
    };
    let root = std::path::Path::new(&directory);
    let listener = AsyncListener::new(Owner::bind(root, 8).unwrap()).unwrap();
    std::fs::write(root.join("ready"), []).unwrap();
    let (socket, _) = bounded(listener.accept()).await.unwrap();
    let packet = bounded(socket.receive()).await.unwrap();
    assert!(matches!(
        packet.message,
        Message::Register { request: 1, .. }
    ));
    socket
        .send(
            &Message::Registered {
                request: 1,
                route: route(),
            },
            &[],
        )
        .await
        .unwrap();
    status(&socket, 1, RouteState::Active).await;
    let mut input = std::io::stdin().lock().lines();
    assert_eq!(input.next().unwrap().unwrap(), "close");
    socket.close();
    std::fs::write(root.join("closed"), []).unwrap();
    assert_eq!(input.next().unwrap().unwrap(), "exit");
    drop(packet); // Native endpoint ownership survives the IPC loss.
}
struct ProcessFixture {
    root: tempfile::TempDir,
    child: std::process::Child,
}
impl ProcessFixture {
    async fn start() -> Self {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "broker::linux::registration_client::tests::t1007_broker_process_fixture",
                "--nocapture",
            ])
            .env("BLENT_T1007_BROKER_FIXTURE", root.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let fixture = Self { root, child };
        until(|| fixture.root.path().join("ready").exists()).await;
        fixture
    }
    fn command(&mut self, command: &str) {
        use std::io::Write;
        writeln!(self.child.stdin.as_mut().unwrap(), "{command}").unwrap();
    }
    fn factory(&self) -> Factory {
        let mut settings = Settings::new(Launch {
            executable: "/bin/false".into(),
            runtime: self.root.path().into(),
            state: self.root.path().into(),
            helper: "/bin/false".into(),
        });
        settings.capacity = 1;
        Factory::new(settings).unwrap()
    }
}
impl Drop for ProcessFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[tokio::test]
async fn t1007_original_broker_exit_recovers_lost_ipc_capacity_and_endpoints() {
    let mut fixture = ProcessFixture::start().await;
    let factory = fixture.factory();
    let endpoints = listeners();
    let addresses = endpoints
        .iter()
        .map(|v| v.local_addr().unwrap())
        .collect::<Vec<_>>();
    let client = factory.start(request(), endpoints).unwrap();
    until(|| client.status().phase == Phase::Active).await;
    fixture.command("close");
    until(|| fixture.root.path().join("closed").exists()).await;
    until(|| client.status().failure.is_some()).await;
    client.close();
    assert!(!client.retired());
    assert!(!client.status().remote_retired);
    assert_eq!(factory.pool.active(), 1);
    assert!(factory.start(request(), listeners()).is_err());
    assert!(addresses
        .iter()
        .all(|address| TcpListener::bind(address).is_err()));
    fixture.command("exit");
    assert!(fixture.child.wait().unwrap().success());
    until(|| client.retired()).await;
    assert!(client.status().remote_retired);
    assert!(!client.status().live());
    assert_eq!(factory.pool.active(), 0);
    assert!(addresses
        .iter()
        .all(|address| TcpListener::bind(address).is_ok()));
}

#[tokio::test]
async fn t1007_broker_death_waits_for_actual_local_native_cleanup() {
    let mut fixture = ProcessFixture::start().await;
    let socket = AsyncSocket::new(
        crate::broker::linux::Socket::connect(&fixture.root.path().join("broker.sock"), unsafe {
            libc::geteuid()
        })
        .unwrap(),
    )
    .unwrap();
    let endpoints = Arc::new(
        Endpoints::validate(
            request().resource,
            listeners().into_iter().map(OwnedFd::from).collect(),
        )
        .unwrap(),
    );
    let shared = Arc::new(Shared::new(Model::new(request()).unwrap()));
    shared.state().broker = Some(socket.original_lifetime().unwrap());
    let message = shared.update(Model::outgoing).unwrap();
    bounded(socket.send(&message, &endpoints.descriptors()))
        .await
        .unwrap();
    for _ in 0..2 {
        let packet = bounded(socket.receive()).await.unwrap();
        shared
            .update(|model| model.receive(packet.message))
            .unwrap();
    }
    assert_eq!(shared.state().model.status().phase, Phase::Active);
    let (release, blocked) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || blocked.recv_timeout(Duration::from_secs(5)).unwrap());
    let pool = Pool::new(1).unwrap();
    let admission = pool.reserve().unwrap();
    let completion = admission.completion();
    admission.retain(Retained {
        thread: retirement_work::Thread::new(thread),
        shared: shared.clone(),
        _endpoints: endpoints,
    });
    fixture.command("close");
    until(|| fixture.root.path().join("closed").exists()).await;
    assert!(bounded(socket.receive()).await.is_err());
    shared.fail(Failure::Native);
    assert!(!completion.is_complete());
    fixture.command("exit");
    assert!(fixture.child.wait().unwrap().success());
    assert!(!completion.is_complete());
    assert!(!shared.state().model.status().remote_retired);
    assert_eq!(pool.active(), 1);
    assert!(pool.reserve().is_err());
    release.send(()).unwrap();
    until(|| completion.is_complete()).await;
    assert_eq!(pool.active(), 0);
}

#[tokio::test]
async fn t1007_dead_original_and_unrelated_process_exit_cannot_release_replacement() {
    let mut fixture = ProcessFixture::start().await;
    let factory = fixture.factory();
    let original = factory.start(request(), listeners()).unwrap();
    until(|| original.status().phase == Phase::Active).await;
    fixture.command("close");
    until(|| fixture.root.path().join("closed").exists()).await;
    fixture.command("exit");
    assert!(fixture.child.wait().unwrap().success());
    until(|| original.retired()).await;
    let listener = AsyncListener::new(Owner::bind(fixture.root.path(), 8).unwrap()).unwrap();
    let replacement = factory.start(request(), listeners()).unwrap();
    let (server, _) = bounded(listener.accept()).await.unwrap();
    let packet = bounded(server.receive()).await.unwrap();
    drop(packet);
    server
        .send(
            &Message::Registered {
                request: 1,
                route: route(),
            },
            &[],
        )
        .await
        .unwrap();
    status(&server, 1, RouteState::Active).await;
    until(|| replacement.status().phase == Phase::Active).await;
    assert!(std::process::Command::new("/bin/true")
        .status()
        .unwrap()
        .success());
    for _ in 0..32 {
        original.close();
        assert!(original.retired());
        assert!(!replacement.retired());
        assert!(replacement.status().live());
    }
    server.close();
    until(|| replacement.status().failure.is_some()).await;
    assert!(!replacement.retired());
    assert_eq!(factory.pool.active(), 1);
    // Exact route retirement only to clean the isolated, still-live parent broker.
    replacement
        .shared
        .update(|model| {
            model.receive(Message::Status {
                request: 1,
                route: route(),
                state: RouteState::Retired,
            })
        })
        .unwrap();
    until(|| replacement.retired()).await;
}

#[test]
fn t1007_unavailable_original_observer_preserves_quarantine() {
    let shared = Arc::new(Shared::new(Model::new(request()).unwrap()));
    shared.update(Model::outgoing);
    shared.fail(Failure::Native);
    let thread = std::thread::spawn(|| ());
    while !thread.is_finished() {
        std::thread::yield_now();
    }
    let endpoints = Arc::new(
        Endpoints::validate(
            request().resource,
            listeners().into_iter().map(OwnedFd::from).collect(),
        )
        .unwrap(),
    );
    let mut retained = Retained {
        thread: retirement_work::Thread::new(thread),
        shared: shared.clone(),
        _endpoints: endpoints,
    };
    for _ in 0..32 {
        assert!(!retained.poll_complete());
    }
    assert!(!shared.state().model.status().remote_retired);
}
