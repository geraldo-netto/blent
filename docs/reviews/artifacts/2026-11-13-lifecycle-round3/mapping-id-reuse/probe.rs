//! Private X11 server-incarnation model; no live native input/server effects.
use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::time::Duration;
use tokio::sync::watch;
mod fixture;

async fn pause() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}
async fn reassigned_id() {
    drop(fixture::Release::new());
    let (_card, cards) = watch::channel(None);
    let (tablet, tablets) = watch::channel(true);
    let (_mode, modes) = watch::channel(true);
    let backend = LinuxBackend::new(cards).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(
        tablets,
        modes,
        InputConfig {
            touch: true,
            pen: false,
            pointer: false,
            ..Default::default()
        },
    ));
    while !fixture::calls().contains("xinput list-props 41") {
        pause().await;
    }
    let calls = fixture::calls();
    tablet.send(false).unwrap();
    while fixture::counts().1 != 1 {
        pause().await;
    }
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    while !native.is_complete() {
        pause().await;
    }
    let pid = std::fs::read_to_string(fixture::child()).unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    println!("{calls}");
    assert!(calls.contains("applied-to-foreign-registration"));
    assert_eq!(fixture::counts(), (1, 1));
}
#[test]
fn numeric_selection_crosses_server_incarnations() {
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run("numeric_selection_crosses_server_incarnations", "0");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(reassigned_id());
}
