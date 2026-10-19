use super::*;
use crate::{
    accessory::{setup::tests::remote_hello, Stream},
    broker::{
        control::{Action, Controller, Device, Limits as ControlLimits},
        session::tests::tls::tablet,
    },
    secure_connection::{self, PeerCertificate},
};
use blent_config::connection::{
    control::{Command as LocalCommand, Notice},
    management::{Credential, Profile, Resource as MediaResource},
    ready,
    trust::{Trust, TrustStorage},
    window as protocol_window,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Control = Controller<u64, u64>;
struct Store;
impl TrustStorage for Store {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
    fn replace(&mut self, _: &[u8]) -> Result<()> {
        Ok(())
    }
}
fn pin(identity: &Identity) -> PeerId {
    let certificate = PeerCertificate::parse(identity.certificate().to_vec()).unwrap();
    PeerId::parse(
        certificate
            .fingerprint()
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect(),
    )
    .unwrap()
}
fn controller(local: PeerId) -> (Control, u64, u64) {
    let mut control = Control::new(local, vec![], ControlLimits::default()).unwrap();
    let client = control.admit_client().unwrap();
    control
        .replace_devices(vec![Device {
            key: 1,
            payload: 1,
            vendor: 1,
            product: 1,
            accessory: false,
        }])
        .unwrap();
    let (revision, target) = control
        .snapshot()
        .iter()
        .find_map(|notice| match notice {
            Notice::Device {
                revision, target, ..
            } => Some((*revision, *target)),
            _ => None,
        })
        .unwrap();
    let id = match control
        .command(client, LocalCommand::Connect { revision, target }, 0)
        .unwrap()
    {
        Action::Start { connection, .. } => connection,
        _ => panic!("expected start"),
    };
    (control, client, id)
}
async fn notice(handle: &mut Handle) -> Event {
    tokio::time::timeout(Duration::from_secs(3), handle.next())
        .await
        .unwrap()
        .unwrap()
}
async fn remote_tls(
    mut remote: tokio::io::DuplexStream,
    tablet: Identity,
    host: Arc<Identity>,
) -> Box<dyn Stream> {
    let mut marker = [0; ready::BYTES];
    remote.read_exact(&mut marker).await.unwrap();
    ready::decode(&marker, true).unwrap();
    remote.write_all(&ready::encode(false)).await.unwrap();
    let host = PeerCertificate::parse(host.certificate().to_vec()).unwrap();
    let mut tls = secure_connection::connect(
        remote,
        &tablet,
        &host,
        blent_config::connection::handshake::Timeout::default(),
    )
    .await
    .unwrap();
    let mut window = [0; protocol_window::BYTES];
    tls.read_exact(&mut window).await.unwrap();
    assert_eq!(protocol_window::decode(&window, true).unwrap(), 4096);
    tls.write_all(&protocol_window::encode(false, 4096).unwrap())
        .await
        .unwrap();
    tls.flush().await.unwrap();
    Box::new(tls)
}
struct VerifiedLink {
    control: Control,
    client: u64,
    id: u64,
    handle: Handle,
    remote: Box<dyn Stream>,
    verified: Verified,
    limits: Limits,
    local: PeerId,
}
async fn verified() -> VerifiedLink {
    let (resource, mut remote) = resource();
    let mut options = settings();
    options.limits.channel_bytes = 4096;
    let host = options.identity.clone();
    let local = pin(&host);
    let (mut control, client, id) = controller(local.clone());
    let mut handle = Handle::start(resource, options).unwrap();
    control.attach(id, handle.native_completion()).unwrap();
    let tablet = Identity::load(IdentityRecord::generate().unwrap()).unwrap();
    tokio::time::timeout(
        Duration::from_secs(3),
        remote_hello(&mut remote, tablet.certificate()),
    )
    .await
    .unwrap();
    let peer = match notice(&mut handle).await {
        Event::Identified(peer) => peer,
        _ => panic!("expected public identity"),
    };
    assert_eq!(peer, pin(&tablet));
    control.identified(id, peer, 1).unwrap();
    let ticket = control
        .snapshot()
        .iter()
        .find_map(|notice| match notice {
            Notice::Link { ticket, .. } => ticket.clone(),
            _ => None,
        })
        .unwrap();
    control
        .command(
            client,
            LocalCommand::Decide {
                connection: id,
                ticket: ticket.clone(),
                allow: true,
                remember: false,
            },
            2,
        )
        .unwrap();
    assert!(handle.submit(Command::Approve(ticket)).is_ok());
    let (notice, remote) = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(notice(&mut handle), remote_tls(remote, tablet, host))
    })
    .await
    .unwrap();
    let (verified, limits) = match notice {
        Event::Verified(verified, limits) => (verified, limits),
        _ => panic!("expected verified TLS"),
    };
    assert_eq!(limits.channel_bytes, 4096);
    VerifiedLink {
        control,
        client,
        id,
        handle,
        remote,
        verified,
        limits,
        local,
    }
}
struct AdmittedLink {
    control: Control,
    id: u64,
    handle: Handle,
    remote: Box<dyn Stream>,
}
impl VerifiedLink {
    fn admit(mut self) -> AdmittedLink {
        let job = self
            .control
            .prepare_authentication(
                self.id,
                &self.verified.ticket,
                self.verified.peer.clone(),
                3,
            )
            .unwrap();
        let mut trust = Trust::open(Store, self.local, 64).unwrap();
        let admitted = self
            .control
            .complete_authority(job.run(&mut trust), 4)
            .unwrap()
            .unwrap();
        let connection = Connection::new(
            self.verified,
            self.control.pairing(self.id).unwrap(),
            &admitted.lease,
            self.limits,
        )
        .unwrap();
        assert!(self.handle.submit(Command::Admit(connection)).is_ok());
        AdmittedLink {
            control: self.control,
            id: self.id,
            handle: self.handle,
            remote: self.remote,
        }
    }
}
async fn completed(completion: &Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
fn display(connect: super::super::super::bridges::Connect) -> Command {
    Command::Register(Offer {
        resource: MediaResource::DisplaySession,
        owner: 17,
        credential: Credential::new([9; 32]),
        profile: Profile::Display,
        connect,
    })
}
async fn admitted_event(handle: &mut Handle) -> Lease {
    let mut registered = None;
    for _ in 0..8 {
        match notice(handle).await {
            Event::Registered { owner, result } => {
                assert_eq!(owner, 17);
                registered = Some(result.unwrap());
            }
            Event::State(lease, State::Admitted) => {
                assert_eq!(registered.as_ref(), Some(&lease));
                return lease;
            }
            _ => (),
        }
    }
    panic!("no admitted resource");
}

#[tokio::test]
async fn t853_full_hello_tls_window_controller_and_local_display_control_flow() {
    let mut link = verified().await.admit();
    let native = link.handle.native_completion();
    let task = link.handle.task_completion();
    let (video, _video_owner) = tokio::io::duplex(64);
    let (control, mut app) = tokio::io::duplex(64);
    let opened = Arc::new(AtomicUsize::new(0));
    let observed = opened.clone();
    let connect = Box::pin(async move {
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(vec![
            Box::new(video) as Box<dyn Stream>,
            Box::new(control) as Box<dyn Stream>,
        ])
    });
    assert!(link.handle.submit(display(connect)).is_ok());
    assert_eq!(opened.load(Ordering::SeqCst), 0);
    let peer = tokio::spawn(tablet(link.remote));
    let lease = admitted_event(&mut link.handle).await;
    let exchange = async {
        app.write_all(b"host control").await.unwrap();
        let mut bytes = [0; 14];
        app.read_exact(&mut bytes).await.unwrap();
        assert_eq!(&bytes, b"tablet control");
        peer.await.unwrap()
    };
    let _peer_stream = tokio::time::timeout(Duration::from_secs(3), exchange)
        .await
        .unwrap();
    assert_eq!(opened.load(Ordering::SeqCst), 1);
    assert!(link.handle.submit(Command::Cancel(lease.clone())).is_ok());
    match notice(&mut link.handle).await {
        Event::State(current, State::Closing) => assert_eq!(current, lease),
        _ => panic!("expected closing"),
    }
    link.handle.stop();
    drop(link.handle);
    completed(&native).await;
    completed(&task).await;
    link.control.native_finished(link.id).unwrap();
    assert_eq!(link.control.collect_retired().unwrap(), vec![link.id]);
}

#[tokio::test]
async fn t853_cancelled_verified_callback_cannot_install_authority_or_release_owner_early() {
    let mut link = verified().await;
    let ticket = link.verified.ticket.clone();
    let native = link.handle.native_completion();
    let task = link.handle.task_completion();
    link.control
        .command(
            link.client,
            LocalCommand::Disconnect {
                connection: link.id,
                ticket: Some(ticket.clone()),
            },
            5,
        )
        .unwrap();
    assert!(link
        .control
        .prepare_authentication(link.id, &ticket, link.verified.peer.clone(), 6)
        .is_err());
    link.handle.stop();
    drop(link.handle);
    assert!(
        !native.is_complete(),
        "dispatcher still owns Verified native resource"
    );
    drop(link.verified);
    completed(&native).await;
    completed(&task).await;
    link.control.native_finished(link.id).unwrap();
    assert_eq!(link.control.collect_retired().unwrap(), vec![link.id]);
}

#[tokio::test]
async fn t853_unexpected_setup_command_during_media_closes_native_owner() {
    let link = verified().await.admit();
    let mut handle = link.handle;
    let native = handle.native_completion();
    let ticket = link.control.lease(link.id).unwrap().ticket.clone();
    assert!(handle.submit(Command::Approve(ticket)).is_ok());
    assert!(tokio::time::timeout(Duration::from_secs(3), handle.next())
        .await
        .unwrap()
        .is_err());
    completed(&native).await;
}

#[tokio::test]
async fn t853_link_command_capacity_returns_exact_unsubmitted_ticket() {
    let (resource, _remote) = resource();
    let handle = Handle::start(resource, settings()).unwrap();
    let peer = PeerId::parse("a".repeat(64)).unwrap();
    for epoch in 1..=8 {
        assert!(handle
            .submit(Command::Approve(Ticket {
                epoch,
                peer: peer.clone()
            }))
            .is_ok());
    }
    let exact = Ticket { epoch: 9, peer };
    match handle
        .submit(Command::Approve(exact.clone()))
        .err()
        .unwrap()
    {
        Command::Approve(returned) => assert_eq!(returned, exact),
        _ => panic!("wrong returned command"),
    }
    let native = handle.native_completion();
    drop(handle);
    completed(&native).await;
}

mod deadline;
