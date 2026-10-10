use super::*;
use blent_config::connection::{
    management::Resource,
    routes::{RouteClaims, RouteId},
};
use std::net::TcpListener;
#[test]
fn t871_failed_retirement_retains_the_only_listener_capability() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let mut endpoints = Endpoints::validate(Resource::Camera, vec![listener.into()]).unwrap();
    let mut claims = RouteClaims::new(1).unwrap();
    endpoints.reserve(&mut claims, 1).unwrap();
    assert!(endpoints
        .retire(
            &mut claims,
            RouteId {
                owner: 2,
                revision: 1
            }
        )
        .is_err());
    assert!(
        TcpListener::bind(address).is_err(),
        "T871 wrong retirement released live endpoint capability"
    );
}

fn root() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
fn connections(root: &Path) -> (Listener, Socket, Socket) {
    let path = root.join("broker.sock");
    let uid = unsafe { libc::geteuid() };
    let listener = Listener::bind(&path, 4).unwrap();
    assert!(listener.accept(uid).unwrap().is_none());
    let client = Socket::connect(&path, uid).unwrap();
    let (server, caller) = listener.accept(uid).unwrap().unwrap();
    assert_eq!(caller.uid, uid);
    assert_eq!(caller.pid, std::process::id());
    (listener, client, server)
}
#[test]
fn t871_native_authenticated_registration_transfers_pinned_display_listeners() {
    use blent_config::connection::{
        management::{Credential, Profile},
        pairing::PeerId,
        registration::Message,
    };
    let root = root();
    let (_listener, client, server) = connections(root.path());
    let video = TcpListener::bind("127.0.0.1:0").unwrap();
    let control = TcpListener::bind("127.0.0.1:0").unwrap();
    let ports = [video.local_addr().unwrap(), control.local_addr().unwrap()];
    let message = Message::Register {
        request: 7,
        peer: PeerId::parse("a".repeat(64)).unwrap(),
        resource: Resource::DisplaySession,
        credential: Credential::new([4; 32]),
        profile: Profile::Display,
    };
    client
        .send(
            &message.encode().unwrap(),
            &[video.as_raw_fd(), control.as_raw_fd()],
        )
        .unwrap();
    let mut buffer = [0; 4096];
    let packet = server.receive(&mut buffer).unwrap().unwrap();
    let decoded = Message::decode(&buffer[..packet.length]).unwrap();
    assert_eq!(decoded.request(), 7);
    assert_eq!(packet.descriptors.len(), decoded.descriptor_count());
    let mut endpoints = Endpoints::validate(Resource::DisplaySession, packet.descriptors).unwrap();
    assert_eq!(endpoints.addresses(), ports);
    let mut claims = RouteClaims::new(2).unwrap();
    let route = endpoints.reserve(&mut claims, 1).unwrap();
    assert!(endpoints.reserve(&mut claims, 2).is_err());
    assert!(endpoints.retire(&mut claims, route).is_err());
    let reply = Message::Registered { request: 7, route }.encode().unwrap();
    server.send(&reply, &[]).unwrap();
    assert!(client
        .receive(&mut buffer)
        .unwrap()
        .unwrap()
        .descriptors
        .is_empty());
    drop(video);
    drop(control);
    drop(client);
    assert!(server.receive(&mut buffer).is_err());
    assert!(TcpListener::bind(ports[0]).is_err()); // caller death does not release retained admission
    claims.revoke(route).unwrap();
    endpoints.retire(&mut claims, route).unwrap();
    assert!(TcpListener::bind(ports[0]).is_ok());
    assert!(endpoints.addresses().is_empty());
    assert!(endpoints.reserve(&mut claims, 2).is_err());
}
#[test]
fn t871_clients_have_distinct_authenticated_lifetimes() {
    let root = root();
    let (listener, first, server) = connections(root.path());
    let uid = unsafe { libc::geteuid() };
    let second = Socket::connect(&root.path().join("broker.sock"), uid).unwrap();
    let (peer, _) = listener.accept(uid).unwrap().unwrap();
    drop(first);
    assert!(server.receive(&mut [0; 4]).is_err());
    second.send(b"live", &[]).unwrap();
    assert_eq!(peer.receive(&mut [0; 4]).unwrap().unwrap().length, 4);
    assert!(second.caller(uid.wrapping_add(1)).is_err());
    let rejected = Socket::connect(&root.path().join("broker.sock"), uid).unwrap();
    assert!(listener.accept(uid.wrapping_add(1)).is_err());
    assert!(rejected.receive(&mut [0; 4]).is_err());
    assert!(Socket::connect(&root.path().join("broker.sock"), uid.wrapping_add(1)).is_err());
}
#[test]
fn t871_invalid_local_socket_selection_fails_before_resource_use() {
    let root = root();
    let path = root.path().join("broker.sock");
    for backlog in [0, 1025, u32::MAX] {
        assert!(Listener::bind(&path, backlog).is_err());
    }
    for path in [
        std::path::PathBuf::from("relative"),
        std::path::PathBuf::from("/".to_owned() + &"x".repeat(108)),
        std::path::PathBuf::from("/bad\0path"),
    ] {
        assert!(Listener::bind(&path, 1).is_err());
        assert!(Socket::connect(&path, unsafe { libc::geteuid() }).is_err());
    }
    assert!(Socket::connect(&path, unsafe { libc::geteuid() }).is_err());
    let _listener = Listener::bind(&path, 1).unwrap();
    assert!(Listener::bind(&path, 1).is_err());
}
#[test]
fn t871_nonlisteners_foreign_addresses_and_duplicate_descriptors_are_rejected() {
    assert!(Endpoints::validate(Resource::DisplaySession, vec![]).is_err());
    assert!(
        Endpoints::validate(Resource::Camera, vec![tempfile::tempfile().unwrap().into()]).is_err()
    );
    let wildcard = TcpListener::bind("0.0.0.0:0").unwrap();
    assert!(Endpoints::validate(Resource::Camera, vec![wildcard.into()]).is_err());
    let loopback = TcpListener::bind("127.0.0.1:0").unwrap();
    let duplicate = loopback.try_clone().unwrap();
    assert!(Endpoints::validate(
        Resource::DisplaySession,
        vec![loopback.into(), duplicate.into()]
    )
    .is_err());
    let root = root();
    let unix = std::os::unix::net::UnixListener::bind(root.path().join("unix")).unwrap();
    assert!(Endpoints::validate(Resource::Camera, vec![unix.into()]).is_err());
    let datagram = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    assert!(Endpoints::validate(Resource::Camera, vec![datagram.into()]).is_err());
}

#[test]
fn t871_shared_port_is_not_an_exclusive_endpoint_capability() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let enabled = 1i32;
    assert_eq!(
        unsafe {
            libc::setsockopt(
                listener.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_REUSEPORT,
                (&enabled as *const i32).cast(),
                4,
            )
        },
        0
    );
    assert!(
        Endpoints::validate(Resource::Camera, vec![listener.into()]).is_err(),
        "T871 shared TCP port accepted as exclusive"
    );
}

#[test]
fn t871_typed_boundary_rejects_malformed_messages_and_unexpected_descriptor_ownership() {
    use blent_config::connection::registration::{Message, Rejection};
    let root = root();
    let (_listener, client, server) = connections(root.path());
    assert!(server.receive_message().unwrap().is_none());
    let rejection = Message::Rejected {
        request: 1,
        reason: Rejection::Invalid,
    };
    client.send_message(&rejection, &[]).unwrap();
    let received = server.receive_message().unwrap().unwrap();
    assert_eq!(received.message.request(), 1);
    assert!(received.descriptors.is_empty());
    let file = tempfile::tempfile().unwrap();
    assert!(client
        .send_message(&rejection, &[file.as_raw_fd()])
        .is_err());
    assert!(client
        .send_message(
            &Message::Rejected {
                request: 0,
                reason: Rejection::Invalid
            },
            &[]
        )
        .is_err());
    client
        .send(&rejection.encode().unwrap(), &[file.as_raw_fd()])
        .unwrap();
    assert!(server.receive_message().is_err());
    client.send(b"bad", &[file.as_raw_fd()]).unwrap();
    assert!(server.receive_message().is_err());
}
