// Isolated review probe. Requires the repository's fail-closed uinput syscall shim.
use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::{path::Path, time::Duration};
use tokio::sync::watch;

fn counts(log: &Path) -> (usize, usize) {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    (text.matches("request 21761\n").count(), text.matches("request 21762\n").count())
}
async fn wait_count(log: &Path, created: usize, destroyed: usize) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while counts(log) != (created, destroyed) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }).await.unwrap();
}
fn intercepted() {
    let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"blent_test_uinput_active".as_ptr()) };
    assert!(!symbol.is_null(), "refuse native device access without fixture");
    let probe: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol) };
    assert_eq!(unsafe { probe() }, 497);
}
async fn inspect(log: &Path) {
    let (tablet, attached) = watch::channel(true);
    let (_mode, mode) = watch::channel(false);
    let (_card, card) = watch::channel(None);
    let backend = LinuxBackend::new(card);
    let config = InputConfig { touch: true, pen: false, pointer: false, ..Default::default() };
    let first = tokio::spawn(backend.follow(attached.clone(), mode.clone(), config.clone()));
    wait_count(log, 1, 0).await;
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    println!("first follow joined: counts={:?}, native_completion_present={}", counts(log), backend.native_completion().is_some());
    let second = tokio::spawn(backend.follow(attached, mode, config));
    wait_count(log, 2, 0).await;
    second.abort();
    assert!(second.await.unwrap_err().is_cancelled());
    println!("replacement follow joined before original device destruction: counts={:?}", counts(log));
    wait_count(log, 2, 2).await;
    println!("all fixture devices eventually destroyed: counts={:?}", counts(log));
    drop(tablet);
}
fn main() {
    intercepted();
    let log = std::path::PathBuf::from(std::env::var_os("BLENT_T497_UINPUT_LOG").unwrap());
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    runtime.block_on(inspect(&log));
    runtime.shutdown_timeout(Duration::from_secs(5));
}
