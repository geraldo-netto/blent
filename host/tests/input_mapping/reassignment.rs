//! T1446: every helper resolves the exact creation name in its current server.
use super::{complete, config, destroyed, fixture, pause, reaped};
use blent::input::{backend::InputBackend, LinuxBackend};
use tokio::sync::watch;

async fn outcome() -> String {
    loop {
        let calls = fixture::calls();
        if calls.contains("properties ") || calls.contains("selection-rejected") {
            return calls;
        }
        pause().await;
    }
}
async fn current_registry() {
    drop(fixture::Release::new());
    let (_card, cards) = watch::channel(None);
    let (tablet, tablets) = watch::channel(true);
    let (_mode, modes) = watch::channel(true);
    let backend = LinuxBackend::new(cards).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablets, modes, config()));
    let calls = outcome().await;
    tablet.send(false).unwrap();
    destroyed(1).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    complete(&native).await;
    reaped().await;
    assert!(
        calls.contains("mapped 42\n"),
        "T1446 numeric mutation followed a foreign registration: {calls}"
    );
    assert!(
        calls.contains("properties 42\n"),
        "T1446 property read followed a foreign registration: {calls}"
    );
    assert!(!calls.contains("mapped 41\n"));
    assert!(!calls.contains("properties 41\n"));
}
async fn ambiguous_registry() {
    drop(fixture::Release::new());
    let (_card, cards) = watch::channel(None);
    let (_tablet, tablets) = watch::channel(true);
    let (_mode, modes) = watch::channel(true);
    let backend = LinuxBackend::new(cards).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablets, modes, config()));
    let calls = outcome().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    reaped().await;
    if !calls.contains("selection-rejected") {
        complete(&native).await;
    }
    assert!(
        calls.contains("selection-rejected"),
        "T1446 bypassed ambiguous-name rejection with a numeric selector"
    );
    assert!(!calls.contains("mapped "));
    assert_eq!(fixture::counts(), (1, 0));
    assert!(!native.is_complete());
    // Generic post-spawn failure remains unknown even though this private
    // fixture knows it rejected before mutation. Its fake payload stays funded
    // through isolated process teardown; the exact command child was reaped.
}
fn run(name: &str, mode: &str, work: impl std::future::Future<Output = ()>) {
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run_reassignment(name, mode);
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(work);
}
#[test]
fn t1446_mutation_and_readback_use_current_creation_after_id_reassignment() {
    run(
        "reassignment::t1446_mutation_and_readback_use_current_creation_after_id_reassignment",
        "changed",
        current_registry(),
    );
}
#[test]
fn t1446_ambiguous_creation_name_retains_unknown_native_outcome() {
    run(
        "reassignment::t1446_ambiguous_creation_name_retains_unknown_native_outcome",
        "ambiguous",
        ambiguous_registry(),
    );
}
