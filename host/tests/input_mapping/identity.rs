//! T1445: a prior native creation cannot satisfy replacement registration.
use super::{complete, config, destroyed, fixture, pause};
use blent::input::{backend::InputBackend, LinuxBackend};
use tokio::sync::watch;

async fn occurrences(needle: &str, count: usize) {
    while fixture::calls().matches(needle).count() < count {
        pause().await;
    }
}
async fn replacement_mapped() -> bool {
    loop {
        let calls = fixture::calls();
        if calls.matches("xinput map-to-output 41 ").count() > 1 {
            return false;
        }
        if calls.contains("xinput list-props 42") {
            return true;
        }
        pause().await;
    }
}
async fn delayed_registration() {
    drop(fixture::Release::new());
    let (card, cards) = watch::channel(None);
    let (tablet, tablets) = watch::channel(true);
    let (_mode, modes) = watch::channel(true);
    let backend = LinuxBackend::new(cards).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablets, modes, config()));
    occurrences("xinput list-props 41", 1).await;
    tablet.send(false).unwrap();
    destroyed(1).await;
    tablet.send(true).unwrap();
    // The private server has returned the original registration after the new
    // kernel creation. Publishing42 changes no tablet/mode/card watch value.
    occurrences("listed\n", 2).await;
    assert_eq!(fixture::counts(), (2, 1));
    std::fs::write(
        std::env::var_os("BLENT_T1445_REGISTRATION").unwrap(),
        "registered",
    )
    .unwrap();
    let automatic = replacement_mapped().await;
    if !automatic {
        // Faulty-control cleanup also proves the new registration is reachable.
        card.send(Some(1)).unwrap();
        occurrences("xinput list-props 42", 1).await;
    }
    let calls = fixture::calls();
    tablet.send(false).unwrap();
    destroyed(2).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    complete(&native).await;
    super::reaped().await;
    assert!(automatic, "T1445 old registration satisfied a new creation");
    assert_eq!(calls.matches("xinput map-to-output 41 ").count(), 1);
    assert!(calls.contains("xinput map-to-output 42 "));
    assert!(!calls.contains("xinput map-to-output 99 "));
    assert_eq!(fixture::counts(), (2, 2));
}
#[test]
fn t1445_replacement_ignores_old_registration_and_maps_new_creation_without_watch_event() {
    let name = "identity::t1445_replacement_ignores_old_registration_and_maps_new_creation_without_watch_event";
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run_registration(name);
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(delayed_registration());
}
