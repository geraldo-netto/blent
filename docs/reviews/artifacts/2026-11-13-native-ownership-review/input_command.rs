use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::{ffi::{c_char, c_void}, path::PathBuf};
use tokio::sync::watch;
unsafe extern "C" { fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void; }
fn intercepted() {
    let symbol = unsafe { dlsym(std::ptr::null_mut(), c"blent_test_uinput_active".as_ptr()) };
    assert!(!symbol.is_null(), "Refuse execution without input shim");
    let probe: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol) };
    assert_eq!(unsafe { probe() }, 497);
}
struct Release(PathBuf);
impl Drop for Release {
    fn drop(&mut self) { std::fs::write(&self.0, "release").unwrap(); }
}
async fn check() {
    let release = Release(std::env::var_os("BLENT_REVIEW_RELEASE").unwrap().into());
    let child = PathBuf::from(std::env::var_os("BLENT_REVIEW_CHILD").unwrap());
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(false);
    let backend = LinuxBackend::new(card.clone()).unwrap();
    let proof = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablet, mode, InputConfig { touch:true, pen:false, pointer:false, ..Default::default() }));
    while !child.exists() { tokio::time::sleep(std::time::Duration::from_millis(1)).await; }
    let pid = std::fs::read_to_string(child).unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    while !proof.is_complete() { tokio::time::sleep(std::time::Duration::from_millis(1)).await; }
    println!("input_native_complete={} original_command_alive={}", proof.is_complete(), PathBuf::from(format!("/proc/{pid}")).exists());
    drop(release);
    while PathBuf::from(format!("/proc/{pid}")).exists() { tokio::time::sleep(std::time::Duration::from_millis(1)).await; }
    println!("original_command_reaped=true");
}
fn main() {
    intercepted();
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(check());
}
