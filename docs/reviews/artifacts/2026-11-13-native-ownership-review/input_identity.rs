use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::{ffi::{c_char, c_void}, panic::AssertUnwindSafe};
use tokio::sync::watch;
unsafe extern "C" { fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void; }
fn intercepted() {
    let symbol = unsafe { dlsym(std::ptr::null_mut(), c"blent_test_uinput_active".as_ptr()) };
    assert!(!symbol.is_null(), "Refuse execution without fail-closed input shim");
    let probe: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol) };
    assert_eq!(unsafe { probe() }, 497);
}
fn check(instance: u32) {
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(false);
    let (_mode, mode) = watch::channel(false);
    let backend = LinuxBackend::new(card).unwrap();
    let proof = backend.native_completion().unwrap();
    let config = InputConfig { instance, touch: false, pen: false, pointer: false, ..Default::default() };
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| drop(backend.follow(tablet, mode, config))));
    println!("instance={instance} no_devices=true unpolled_follow_panicked={}", result.is_err());
    drop(backend);
    while !proof.is_complete() { std::thread::sleep(std::time::Duration::from_millis(1)); }
}
fn main() {
    intercepted();
    for instance in [0, 4095, 4096, 65536, u32::MAX] { check(instance); }
}
