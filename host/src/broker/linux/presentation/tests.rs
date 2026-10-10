use super::*;
use crate::broker::linux::{
    async_io::{AsyncListener, AsyncSocket},
    owner::Owner,
};
use blent_config::connection::{
    control::{Command, Notice, State},
    control_view::Model,
    pairing::{PeerId, Ticket},
    registration::{Message, Rejection},
    wire::MAX_COUNTER,
};
use std::os::{fd::AsFd, unix::fs::PermissionsExt};

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
    .await;
}
fn peer(value: u64) -> PeerId {
    PeerId::parse(format!("{value:064x}")).unwrap()
}
fn ticket() -> Ticket {
    Ticket {
        epoch: 3,
        peer: peer(2),
    }
}
fn notices(revision: u64, state: State) -> Vec<Notice> {
    vec![
        Notice::Begin {
            revision,
            local: peer(1),
        },
        Notice::Device {
            revision,
            target: 1,
            vendor: 0x18d1,
            product: 0x2d00,
            accessory: true,
        },
        Notice::Link {
            revision,
            connection: 2,
            ticket: Some(ticket()),
            state,
            trusted: state == State::Authenticated,
        },
        Notice::Saved {
            revision,
            peer: peer(4),
        },
        Notice::End { revision },
    ]
}
pub(super) fn view() -> View {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    let mut snapshot = None;
    for notice in notices(1, State::Pending) {
        snapshot = assembler.push(notice).unwrap();
    }
    Model::default().publish(snapshot.unwrap()).unwrap()
}
async fn send_snapshot(socket: &AsyncSocket, request: u64, revision: u64, state: State) {
    for notice in notices(revision, state) {
        bounded(socket.send(&Message::Report { request, notice }, &[]))
            .await
            .unwrap();
    }
}
async fn request(socket: &AsyncSocket) -> (u64, Command) {
    let message = bounded(socket.receive()).await.unwrap();
    match message.message {
        Message::Control { request, command } => (request, command),
        _ => panic!("unexpected fixture request"),
    }
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
        let mut settings = Settings::new(Launch {
            executable: "/bin/false".into(),
            runtime: root.path().into(),
            state: root.path().into(),
            helper: root.path().join("unused"),
        });
        settings.poll = Duration::from_secs(60);
        let factory = Factory::new(settings).unwrap();
        Self {
            _root: root,
            listener,
            factory,
        }
    }
    async fn connected(&self) -> (Handle, AsyncSocket) {
        let client = self.factory.start().unwrap();
        let (server, _) = bounded(self.listener.accept()).await.unwrap();
        (client, server)
    }
    async fn ready(&self) -> (Handle, AsyncSocket, View) {
        let (client, server) = self.connected().await;
        let (id, command) = request(&server).await;
        assert_eq!(command, Command::Inspect);
        send_snapshot(&server, id, 1, State::Pending).await;
        until(|| client.status().phase == Phase::Ready).await;
        let view = client.status().view.unwrap();
        (client, server, view)
    }
}

#[tokio::test]
async fn t901_explicit_control_waits_for_complete_snapshot_and_durable_results() {
    let fixture = Fixture::new();
    let (client, server, view) = fixture.ready().await;
    assert_eq!(view.snapshot().computer(), &peer(1));
    let command = Command::Decide {
        connection: 2,
        ticket: ticket(),
        allow: true,
        remember: true,
    };
    client
        .submit(view.prepare(command.clone()).unwrap())
        .unwrap();
    let (id, received) = request(&server).await;
    assert_eq!(received, command);
    assert_eq!(
        client.status().view.unwrap().snapshot().links()[&2].state,
        State::Pending
    );
    send_snapshot(&server, id, 2, State::Authenticated).await;
    until(|| {
        client
            .status()
            .view
            .is_some_and(|v| v.snapshot().revision() == 2)
    })
    .await;
    let view = client.status().view.unwrap();
    assert!(view.snapshot().links()[&2].trusted);
    client
        .submit(view.prepare(Command::Revoke { peer: peer(4) }).unwrap())
        .unwrap();
    let (id, received) = request(&server).await;
    assert_eq!(received, Command::Revoke { peer: peer(4) });
    server
        .send(
            &Message::Rejected {
                request: id,
                reason: Rejection::Failed,
            },
            &[],
        )
        .await
        .unwrap();
    until(|| client.status().message.is_some()).await;
    assert!(client.status().view.is_none());
    assert!(client.status().message.unwrap().contains("Failed"));
    client.close();
    until(|| client.retired()).await;
    assert_eq!(client.status().phase, Phase::Closed);
}

#[tokio::test]
async fn t901_control_queue_is_bounded_and_close_does_not_wait_for_reply() {
    let fixture = Fixture::new();
    let (client, server, view) = fixture.ready().await;
    let action = view.prepare(Command::Inspect).unwrap();
    client.submit(action.clone()).unwrap();
    let _ = request(&server).await;
    for _ in 0..fixture.factory.settings.queue {
        client.submit(action.clone()).unwrap();
    }
    assert_eq!(client.submit(action.clone()), Err(SubmitError::Full));
    assert!(fixture.factory.start().is_err());
    client.close();
    client.close();
    assert_eq!(client.submit(action), Err(SubmitError::Closed));
    until(|| client.retired()).await;
    assert!(bounded(server.receive()).await.is_err());
}

#[tokio::test]
async fn t901_dropped_handle_retires_and_old_actions_cannot_cross_reconnection() {
    let fixture = Fixture::new();
    let (first, server, view) = fixture.ready().await;
    let old = view
        .prepare(Command::Decide {
            connection: 2,
            ticket: ticket(),
            allow: true,
            remember: true,
        })
        .unwrap();
    let retired = first.completion.clone();
    drop(first);
    until(|| retired.is_complete()).await;
    assert!(bounded(server.receive()).await.is_err());
    let (second, server, _) = fixture.ready().await;
    second.submit(old).unwrap();
    until(|| second.status().message.is_some()).await;
    assert!(second
        .status()
        .message
        .unwrap()
        .contains("another broker lifetime"));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), server.receive())
            .await
            .is_err()
    );
    second.close();
    until(|| second.retired()).await;
}

#[tokio::test]
async fn t901_unavailable_startup_and_peer_loss_finish_without_false_retirement() {
    let mut fixture = Fixture::new();
    fixture.factory.settings.startup = Duration::from_millis(20);
    let (client, server) = fixture.connected().await;
    let _ = request(&server).await;
    drop(server);
    until(|| client.retired()).await;
    assert_eq!(client.status().phase, Phase::Failed);
    assert!(client.status().view.is_none());
    assert_eq!(
        client.submit(view().prepare(Command::Inspect).unwrap()),
        Err(SubmitError::Closed)
    );
    client.close();
    assert_eq!(client.status().phase, Phase::Failed);
    drop(fixture.listener);
    let client = fixture.factory.start().unwrap();
    until(|| client.retired()).await;
    assert_eq!(client.status().phase, Phase::Failed);
}

#[tokio::test]
async fn t901_reply_order_wrong_direction_and_descriptor_injection_fail_closed() {
    for mode in 0..4 {
        let fixture = Fixture::new();
        let (client, server) = fixture.connected().await;
        let (id, _) = request(&server).await;
        malformed(&server, id, mode).await;
        until(|| client.retired()).await;
        assert_eq!(client.status().phase, Phase::Failed);
        assert!(client.status().view.is_none());
    }
}
async fn malformed(server: &AsyncSocket, request: u64, mode: u8) {
    let message = match mode {
        0 => Message::Report {
            request: request + 1,
            notice: Notice::End { revision: 1 },
        },
        1 => Message::Control {
            request,
            command: Command::Inspect,
        },
        2 => Message::Report {
            request,
            notice: Notice::End { revision: 1 },
        },
        _ => Message::Register {
            request,
            peer: peer(2),
            resource: blent_config::connection::management::Resource::DisplaySession,
            credential: blent_config::connection::management::Credential::new([9; 32]),
            profile: blent_config::connection::management::Profile::Display,
        },
    };
    let file = tempfile::tempfile().unwrap();
    let descriptors = if mode == 3 {
        vec![file.as_fd(), file.as_fd()]
    } else {
        vec![]
    };
    bounded(server.send(&message, &descriptors)).await.unwrap();
}

#[tokio::test]
async fn t901_partial_snapshot_uses_one_absolute_transaction_deadline() {
    let mut fixture = Fixture::new();
    fixture.factory.settings.transaction = Duration::from_millis(100);
    let (client, server) = fixture.connected().await;
    let (id, _) = request(&server).await;
    server
        .send(
            &Message::Report {
                request: id,
                notice: notices(1, State::Pending).remove(0),
            },
            &[],
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(60)).await;
    server
        .send(
            &Message::Report {
                request: id,
                notice: notices(1, State::Pending).remove(1),
            },
            &[],
        )
        .await
        .unwrap();
    assert!(client.status().view.is_none());
    until(|| client.retired()).await;
    assert_eq!(client.status().phase, Phase::Failed);
}

#[tokio::test]
async fn t901_periodic_inspection_refreshes_without_new_threads_or_user_action() {
    let mut fixture = Fixture::new();
    fixture.factory.settings.poll = Duration::from_millis(5);
    let (client, server, _) = fixture.ready().await;
    let (id, command) = request(&server).await;
    assert_eq!(command, Command::Inspect);
    send_snapshot(&server, id, 2, State::Closing).await;
    until(|| {
        client
            .status()
            .view
            .is_some_and(|v| v.snapshot().revision() == 2)
    })
    .await;
    client.close();
    until(|| client.retired()).await;
}

#[test]
fn t901_settings_and_request_counters_reject_invalid_boundaries() {
    let launch = Launch {
        executable: "/bin/false".into(),
        runtime: "/tmp".into(),
        state: "/tmp".into(),
        helper: "/bin/false".into(),
    };
    for value in [0, 33, usize::MAX] {
        let mut settings = Settings::new(launch.clone());
        settings.queue = value;
        assert!(Factory::new(settings).is_err());
    }
    for duration in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        for field in 0..3 {
            let mut settings = Settings::new(launch.clone());
            match field {
                0 => settings.startup = duration,
                1 => settings.transaction = duration,
                _ => settings.poll = duration,
            }
            assert!(Factory::new(settings).is_err());
        }
    }
    let mut settings = Settings::new(launch);
    settings.limits.devices = 0;
    assert!(Factory::new(settings).is_err());
    let mut counter = MAX_COUNTER - 1;
    assert_eq!(exchange::next_request(&mut counter).unwrap(), MAX_COUNTER);
    assert!(exchange::next_request(&mut counter).is_err());
    assert_eq!(counter, MAX_COUNTER);
}

#[test]
fn t901_shared_status_bounds_errors_and_rejects_late_activation_after_close() {
    let shared = Shared::new();
    shared.publish(Phase::Ready, Some(view()), Some("ø".repeat(1024)));
    assert_eq!(
        shared
            .state
            .lock()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .chars()
            .count(),
        512
    );
    shared.stop();
    shared.publish(Phase::Ready, Some(view()), None);
    assert_eq!(shared.state.lock().unwrap().phase, Phase::Closing);
    assert!(shared.state.lock().unwrap().view.is_none());
    shared.finish(Err(anyhow::anyhow!("failed")));
    shared.stop();
    assert_eq!(shared.state.lock().unwrap().phase, Phase::Failed);
}

#[tokio::test]
async fn t901_factory_admission_waits_for_real_native_thread_retirement() {
    let fixture = Fixture::new();
    let admission = fixture.factory.pool.reserve().unwrap();
    let completion = admission.completion();
    let (release, wait) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || wait.recv_timeout(Duration::from_secs(3)).unwrap());
    admission.retain(retirement_work::Thread::new(thread));
    assert!(!completion.is_complete());
    assert!(fixture.factory.start().is_err());
    release.send(()).unwrap();
    until(|| completion.is_complete()).await;
    let client = fixture.factory.start().unwrap();
    client.close();
    until(|| client.retired()).await;
}

#[tokio::test]
async fn t901_production_broker_snapshot_runs_with_an_isolated_synthetic_inventory() {
    use crate::broker::linux::server::{Server, Settings as BrokerSettings};
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join("inventory-helper");
    std::fs::write(&helper, b"#!/usr/bin/python3\nimport sys\nif sys.argv[1] != '--list':\n    sys.exit(7)\nsys.stdout.buffer.write(bytes.fromhex('424c554900000001000100000000000001030100123400070200000000000000'))\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let options = BrokerSettings::new(
        root.path().join("runtime"),
        root.path().join("state"),
        helper.clone(),
    );
    let mut server = bounded(Server::open(options.clone())).await.unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        tokio::select! { result = server.run() => result.unwrap(), _ = stopped => () }
        server.shutdown(Duration::from_secs(2)).await.unwrap();
    });
    let mut settings = Settings::new(Launch {
        executable: "/bin/false".into(),
        runtime: options.runtime,
        state: options.state,
        helper,
    });
    settings.poll = Duration::from_millis(10);
    let factory = Factory::new(settings).unwrap();
    let client = factory.start().unwrap();
    until(|| {
        client
            .status()
            .view
            .is_some_and(|v| v.snapshot().devices().len() == 1)
    })
    .await;
    let observed = client.status().view.unwrap();
    let device = observed.snapshot().devices().values().next().unwrap();
    assert_eq!((device.vendor, device.product), (0x1234, 7));
    assert!(observed.snapshot().links().is_empty());
    assert!(observed.snapshot().saved().is_empty());
    client.close();
    until(|| client.retired()).await;
    stop.send(()).unwrap();
    bounded(task).await.unwrap();
}
