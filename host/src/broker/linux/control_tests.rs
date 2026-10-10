use super::*;
use async_io::{AsyncListener, AsyncSocket};
use blent_config::connection::{
    control::{Command, Notice, State},
    pairing::{PeerId, Ticket},
};
use owner::Owner;
use std::{os::unix::fs::PermissionsExt, time::Duration};

#[tokio::test]
async fn t877_authenticated_control_keeps_connection_ticket_and_snapshot_identity() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let listener = AsyncListener::new(Owner::bind(root.path(), 2).unwrap()).unwrap();
        let client = AsyncSocket::new(
            Socket::connect(&root.path().join("broker.sock"), unsafe { libc::geteuid() }).unwrap(),
        )
        .unwrap();
        let (server, _) = listener.accept().await.unwrap();
        let ticket = Ticket {
            epoch: 49,
            peer: PeerId::parse("cd".repeat(32)).unwrap(),
        };
        let command = Command::Decide {
            connection: 31,
            ticket: ticket.clone(),
            allow: true,
            remember: false,
        };
        client
            .send(
                &Message::Control {
                    request: 1,
                    command: command.clone(),
                },
                &[],
            )
            .await
            .unwrap();
        let received = server.receive().await.unwrap();
        assert!(received.descriptors.is_empty());
        assert!(
            received.message
                == Message::Control {
                    request: 1,
                    command
                }
        );
        let notice = Notice::Link {
            revision: 23,
            connection: 31,
            ticket: Some(ticket),
            state: State::Pending,
            trusted: false,
        };
        server
            .send(
                &Message::Report {
                    request: 1,
                    notice: notice.clone(),
                },
                &[],
            )
            .await
            .unwrap();
        let received = client.receive().await.unwrap();
        assert!(received.descriptors.is_empty());
        assert!(received.message == Message::Report { request: 1, notice });
    })
    .await
    .unwrap();
}
