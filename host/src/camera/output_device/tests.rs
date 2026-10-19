use super::*;
use native::Controls;
use std::{cell::RefCell, io, os::fd::BorrowedFd};

mod corpus;
mod oracle;
mod virtual_driver;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Format,
    Read,
    Write,
}
type ImageChange = fn(&mut abi::Format, u32);
type RateChange = fn(&mut abi::Parameters, u32);
struct Control {
    fail: Option<Stage>,
    image: Option<(ImageChange, u32)>,
    rate: abi::Rate,
    rate_kind: u32,
    change: Option<(RateChange, u32)>,
    events: RefCell<Vec<Stage>>,
}
impl Default for Control {
    fn default() -> Self {
        Self {
            fail: None,
            image: None,
            change: None,
            rate: abi::Rate {
                capability: 0x1000,
                period: abi::Fraction {
                    numerator: 1,
                    denominator: 5,
                },
                ..abi::Rate::default()
            },
            rate_kind: 2,
            events: RefCell::new(Vec::with_capacity(3)),
        }
    }
}
impl Controls for Control {
    fn set_format(&self, _: BorrowedFd<'_>, format: &mut abi::Format) -> io::Result<()> {
        self.enter(Stage::Format)?;
        if let Some((change, value)) = self.image {
            change(format, value);
        }
        Ok(())
    }
    fn get_rate(&self, _: BorrowedFd<'_>, parameters: &mut abi::Parameters) -> io::Result<()> {
        self.enter(Stage::Read)?;
        parameters.kind = self.rate_kind;
        parameters.timing.output = self.rate;
        Ok(())
    }
    fn set_rate(&self, _: BorrowedFd<'_>, parameters: &mut abi::Parameters) -> io::Result<()> {
        self.enter(Stage::Write)?;
        if let Some((change, value)) = self.change {
            change(parameters, value);
        }
        Ok(())
    }
}
impl Control {
    fn enter(&self, stage: Stage) -> io::Result<()> {
        self.events.borrow_mut().push(stage);
        if self.fail == Some(stage) {
            Err(io::Error::other("T1143 injected ioctl failure"))
        } else {
            Ok(())
        }
    }
}
fn profile(width: u32, height: u32, fps: u32) -> CameraProfile {
    CameraProfile {
        width,
        height,
        fps,
        ..CameraProfile::default()
    }
}
fn spec() -> Spec {
    Spec::from_profile(&profile(320, 240, 30)).unwrap()
}

#[test]
fn t1143_fixed_rate_and_each_native_failure_stop_before_later_effects() {
    let file = File::open("/dev/null").unwrap();
    for (stage, length) in [(Stage::Format, 1), (Stage::Read, 2), (Stage::Write, 3)] {
        let control = Control {
            fail: Some(stage),
            ..Control::default()
        };
        assert!(configure_with(&control, file.try_clone().unwrap(), spec()).is_err());
        assert_eq!(control.events.borrow().len(), length);
    }
    let mut control = Control::default();
    control.rate.period.denominator = 30;
    control.rate.capability = 0;
    configure_with(&control, file.try_clone().unwrap(), spec()).unwrap();
    assert_eq!(control.events.borrow().len(), 2);
    control.events.borrow_mut().clear();
    control.rate.period.denominator = 5;
    assert!(configure_with(&control, file.try_clone().unwrap(), spec()).is_err());
    assert_eq!(control.events.borrow().len(), 2);
}

#[test]
fn t1143_native_non_video_capability_rejects_without_descriptor_loss() {
    let file = File::open("/dev/null").unwrap();
    assert!(configure_blocking(file.try_clone().unwrap(), spec()).is_err());
    assert!(native::Ioctl
        .get_rate(file.as_fd(), &mut abi::Parameters::new(30))
        .is_err());
    assert!(native::Ioctl
        .set_rate(file.as_fd(), &mut abi::Parameters::new(30))
        .is_err());
    assert!(!file.metadata().unwrap().is_file());
}

fn reusable(file: &File) -> bool {
    use std::os::fd::AsRawFd;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) }, 0);
        return true;
    }
    assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
    false
}

#[test]
fn t1143_owned_capability_handoff_and_each_failure_preserve_kernel_lease() {
    use std::os::fd::AsRawFd;
    let root = tempfile::tempdir().unwrap();
    for fail in [
        None,
        Some(Stage::Format),
        Some(Stage::Read),
        Some(Stage::Write),
    ] {
        let path = root.path().join("original");
        let original = File::create(&path).unwrap();
        let probe = File::open(&path).unwrap();
        assert_eq!(
            unsafe { libc::flock(original.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let control = Control {
            fail,
            ..Control::default()
        };
        let result = configure_with(&control, original, spec());
        assert_eq!(result.is_err(), fail.is_some());
        if let Ok(prepared) = result {
            assert!(!reusable(&probe));
            let (owned, contract) = prepared.into_parts();
            assert!(!reusable(&probe));
            assert_eq!(contract.frame_bytes(), 115200);
            drop(owned);
        }
        assert!(
            reusable(&probe),
            "T1143 retained original file after actual settlement"
        );
    }
}
