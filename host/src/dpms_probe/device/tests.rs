use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static CLOSED: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    query: c_int,
    capable: c_int,
    status: c_int,
    level: u16,
    enabled: u8,
}
unsafe extern "C" fn open(_: *const c_char) -> *mut c_void {
    Box::into_raw(Box::new(Fixture {
        query: 1,
        capable: 1,
        status: 1,
        level: 0,
        enabled: 1,
    }))
    .cast()
}
unsafe extern "C" fn closed(display: *mut c_void) -> c_int {
    drop(unsafe { Box::from_raw(display.cast::<Fixture>()) });
    CLOSED.fetch_add(1, Ordering::AcqRel);
    0
}
unsafe extern "C" fn query(display: *mut c_void, _: *mut c_int, _: *mut c_int) -> c_int {
    unsafe { (*display.cast::<Fixture>()).query }
}
unsafe extern "C" fn capable(display: *mut c_void) -> c_int {
    unsafe { (*display.cast::<Fixture>()).capable }
}
unsafe extern "C" fn info(display: *mut c_void, level: *mut u16, enabled: *mut u8) -> c_int {
    let state = unsafe { &*display.cast::<Fixture>() };
    unsafe {
        level.write(state.level);
        enabled.write(state.enabled);
    }
    state.status
}
fn device() -> Device {
    Device::with_api(
        Api {
            open,
            close: closed,
            query,
            capable,
            info,
        },
        None,
    )
    .unwrap()
}
#[test]
fn t1013_native_lifetime_closes_once_and_reports_each_dpms_transition() {
    let before = CLOSED.load(Ordering::Acquire);
    let device = device();
    for level in 0..=3 {
        unsafe {
            (*device.display.as_ptr().cast::<Fixture>()).level = level;
        }
        assert_eq!(
            device.sample().unwrap(),
            DisplayObservation {
                supported: true,
                on: Some(level == 0)
            }
        );
    }
    unsafe {
        (*device.display.as_ptr().cast::<Fixture>()).enabled = 0;
        (*device.display.as_ptr().cast::<Fixture>()).level = u16::MAX;
    }
    assert_eq!(device.sample().unwrap().on, Some(true)); // Disabled level is undefined.
    drop(device);
    assert_eq!(CLOSED.load(Ordering::Acquire), before + 1);
}
#[test]
fn t1013_native_rejection_and_unsupported_capability_never_supply_false_state() {
    let device = device();
    for field in 0..5 {
        let state = unsafe { &mut *device.display.as_ptr().cast::<Fixture>() };
        state.query = 1;
        state.capable = 1;
        state.status = 1;
        state.enabled = 1;
        state.level = 0;
        match field {
            0 => state.query = 2,
            1 => state.capable = -1,
            2 => state.status = 0,
            3 => state.enabled = 2,
            _ => state.level = u16::MAX,
        }
        assert!(device.sample().is_err());
    }
    let state = unsafe { &mut *device.display.as_ptr().cast::<Fixture>() };
    state.query = 0;
    assert_eq!(
        device.sample().unwrap(),
        DisplayObservation {
            supported: false,
            on: None
        }
    );
    state.query = 1;
    state.capable = 0;
    assert!(!device.sample().unwrap().supported);
}
#[test]
fn t1013_bounded_native_boolean_and_power_level_properties_reject_invalid_values() {
    for value in [c_int::MIN, -1, 0, 1, 2, c_int::MAX] {
        assert_eq!(boolean(value).is_ok(), (0..=1).contains(&value));
    }
    for level in 0..=u16::MAX {
        assert_eq!(observation(true, level).is_ok(), level <= 3);
        assert_eq!(observation(false, level).unwrap().on, Some(true));
    }
}
unsafe extern "C" fn unavailable(_: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}
#[test]
fn t1013_unavailable_display_fails_before_handle_exposure() {
    assert!(Device::with_api(
        Api {
            open: unavailable,
            close: closed,
            query,
            capable,
            info
        },
        None
    )
    .is_err());
}
// The normal producer loop can be exercised without touching a real desktop.
pub(in crate::dpms_probe) fn producer_device() -> Device {
    device()
}
