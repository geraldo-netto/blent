use super::*;
use crate::input::{handle_connection, Controllers, InputSocket, PendingInput};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

async fn connect(sink: Arc<dyn InputSink>) -> (InputSocket, tokio::task::JoinHandle<Result<()>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (server, _) = listener.accept().await.unwrap();
    let task = tokio::spawn(handle_connection(
        PendingInput::new(server),
        InputConfig {
            token: Some("t1402".into()),
            ..Default::default()
        },
        None,
        watch::channel(false).0,
        Default::default(),
        Arc::new(Controllers::new(sink)),
        Arc::new(tokio::sync::Notify::new()),
    ));
    let (mut client, _) = tokio_tungstenite::client_async("ws://localhost", client)
        .await
        .unwrap();
    client
        .send(Message::Text(r#"{"type":"auth","token":"t1402"}"#.into()))
        .await
        .unwrap();
    client.next().await.unwrap().unwrap();
    (client, task)
}
async fn command(client: &mut InputSocket, command: Command) {
    client
        .send(Message::Text(
            serde_json::json!({"type":"direct_input","command":command}).to_string().into(),
        ))
        .await
        .unwrap();
}
async fn stopped(task: &mut tokio::task::JoinHandle<Result<()>>) -> bool {
    let completed = tokio::time::timeout(std::time::Duration::from_secs(1), &mut *task).await;
    match completed {
        Ok(result) => {
            result.unwrap().unwrap();
            true
        }
        Err(_) => {
            task.abort();
            let _ = task.await;
            false
        }
    }
}

#[tokio::test]
async fn t1402_peer_close_revokes_blocked_native_save() {
    let (backend, _, gate, observe) = held();
    let _release = Release(gate.clone());
    let (mut client, mut task) = connect(backend.sink()).await;
    command(&mut client, Command::Negotiate { version: 1 }).await;
    entered(observe).await;
    client.close(None).await.unwrap();
    let closed = stopped(&mut task).await;
    let proof = backend.completion();
    drop(backend);
    let retained = !proof.is_complete();
    gate.release();
    retired(proof).await;
    assert!(closed, "T1402 close waited for blocked native save");
    assert!(retained, "T1402 close forgot original native worker");
}

#[tokio::test]
async fn t1402_read_ahead_retains_wire_order() {
    let (backend, evidence, gate, observe) = held();
    let _release = Release(gate.clone());
    let (mut client, mut task) = connect(backend.sink()).await;
    command(&mut client, Command::Negotiate { version: 1 }).await;
    entered(observe).await;
    command(&mut client, mouse(Phase::Down, Some(Button::Left), 0.5)).await;
    command(&mut client, mouse(Phase::Up, Some(Button::Left), 0.5)).await;
    gate.release();
    client.next().await.unwrap().unwrap();
    observed(&evidence, "mouse", 2).await;
    client.close(None).await.unwrap();
    assert!(stopped(&mut task).await);
    let proof = backend.completion();
    drop(backend);
    retired(proof).await;
    let calls = evidence.calls.lock().unwrap();
    let mouse: Vec<_> = calls
        .iter()
        .filter(|call| call.starts_with("mouse"))
        .collect();
    assert!(mouse[0].ends_with("Down(Left)"));
    assert!(mouse[1].ends_with("Up(Left)"));
}

#[tokio::test]
async fn t1402_read_ahead_overflow_revokes_without_dispatching_buffered_frames() {
    let (backend, evidence, gate, observe) = held();
    let _release = Release(gate.clone());
    let (mut client, mut task) = connect(backend.sink()).await;
    command(&mut client, Command::Negotiate { version: 1 }).await;
    entered(observe).await;
    for _ in 0..17 {
        command(&mut client, mouse(Phase::Move, None, 0.5)).await;
    }
    let closed = stopped(&mut task).await;
    let proof = backend.completion();
    drop(backend);
    gate.release();
    retired(proof).await;
    assert!(closed, "T1402 unbounded pending socket frames");
    assert!(!evidence
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| call.starts_with("mouse")));
}
