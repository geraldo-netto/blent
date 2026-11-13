//! Private protocol counterexample; no live Xorg/KWin or uinput devices.
use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::time::Duration;
use tokio::sync::watch;
mod fixture;

async fn pause() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}
async fn readback(count: usize, id: u32) {
    let needle = format!("xinput list-props {id}");
    while fixture::calls().matches(&needle).count() < count {
        pause().await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
}
async fn destroyed(count: usize) {
    while fixture::counts().1 < count {
        pause().await;
    }
}
async fn complete(proof: &blent_config::retirement::Completion) {
    while !proof.is_complete() {
        pause().await;
    }
}
async fn stale_registration() {
    drop(fixture::Release::new());
    let (card, cards) = watch::channel(None);
    let (tablet, tablets) = watch::channel(true);
    let (_mode, modes) = watch::channel(true);
    let backend = LinuxBackend::new(cards).unwrap();
    let native = backend.native_completion().unwrap();
    let config = InputConfig {
        touch: true,
        pen: false,
        pointer: false,
        ..Default::default()
    };
    let task = tokio::spawn(backend.follow(tablets, modes, config));
    readback(1, 41).await;
    tablet.send(false).unwrap();
    destroyed(1).await;
    tablet.send(true).unwrap();
    readback(2, 41).await;
    let after_recreation = fixture::counts();
    let before = fixture::calls();
    let release = std::env::var("BLENT_T1439_RELEASE").unwrap();
    std::fs::write(format!("{release}.new"), "server registration 42").unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await;
    let unchanged = fixture::calls() == before;
    card.send(Some(1)).unwrap();
    readback(1, 42).await;
    let after_event = fixture::calls();
    tablet.send(false).unwrap();
    destroyed(2).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    complete(&native).await;
    let pid = std::fs::read_to_string(fixture::child()).unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    println!("after_recreation={after_recreation:?} commands_unchanged_after_new_registration={unchanged} new_id_mapped_after_external_event={}", after_event.contains("map-to-output 42"));
    println!("{after_event}");
    assert_eq!(after_recreation, (2, 1));
    assert!(unchanged);
    assert!(!before.contains("map-to-output 42"));
    assert!(after_event.contains("map-to-output 42"));
    assert_eq!(fixture::counts(), (2, 2));
}
#[test]
fn stale_server_registration_satisfies_new_creation_mapping() {
    let name = "stale_server_registration_satisfies_new_creation_mapping";
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run(name, "0");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(stale_registration());
}
