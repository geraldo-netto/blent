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
async fn check() {
    let device = PathBuf::from(std::env::var_os("BLENT_T497_UINPUT_FILE").unwrap());
    let gate = PathBuf::from(std::env::var_os("BLENT_REVIEW_WRITE_FAIL").unwrap());
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(false);
    let backend = LinuxBackend::new(card).unwrap();
    let proof = backend.native_completion().unwrap();
    let sink = backend.sink();
    let task = tokio::spawn(backend.follow(tablet, mode, InputConfig { touch:true, pen:false, pointer:false, ..Default::default() }));
    while std::fs::metadata(&device).unwrap().len() == 0 {
        sink.touch((0.25,0.25,0.5),0,0);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    let first = std::fs::metadata(&device).unwrap().len();
    sink.release_all();
    let released = std::fs::metadata(&device).unwrap().len();
    println!("successful_release_added_bytes={}", released-first);
    sink.touch((0.25,0.25,0.5),0,0);
    let down = std::fs::metadata(&device).unwrap().len();
    std::fs::write(&gate,"fail").unwrap();
    sink.touch((0.25,0.25,0.5),1,0);
    std::fs::remove_file(gate).unwrap();
    sink.release_all();
    let after = std::fs::metadata(&device).unwrap().len();
    println!("failed_up_then_recovered_release_added_bytes={}", after-down);
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop((sink,backend));
    while !proof.is_complete() { tokio::time::sleep(std::time::Duration::from_millis(1)).await; }
}
fn main() {
    intercepted();
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(check());
}
