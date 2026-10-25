//! T1090 constructor fault checks run with a private process file-limit scope.
use super::*;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

#[test]
fn t1090_failed_runtime_destroys_listeners_before_releasing_admission() {
    if std::env::var_os("BLENT_T1090_RUNTIME_FAILURE_CHILD").is_some() {
        runtime_failure();
        return;
    }
    let full = concat!(
        module_path!(),
        "::t1090_failed_runtime_destroys_listeners_before_releasing_admission"
    );
    let name = full.split_once("::").unwrap().1;
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("BLENT_T1090_RUNTIME_FAILURE_CHILD", "1")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn runtime_failure() {
    let admission = Pool::new(1).unwrap().reserve().unwrap();
    let proof = admission.completion();
    let remote = TcpListener::bind("127.0.0.1:0").unwrap();
    let local = TcpListener::bind("127.0.0.1:0").unwrap();
    let addresses = [remote.local_addr().unwrap(), local.local_addr().unwrap()];
    let target = Target {
        adb: "not-invoked".into(),
        serial: "original".into(),
        remote: format!("tcp:{}", addresses[0].port()),
        local: format!("tcp:{}", addresses[1].port()),
    };
    let endpoints = Endpoints {
        _remote: remote,
        _local: local,
        funding: proof.clone(),
    };
    let limit = NoFiles::enter();
    let mut future = Box::pin(Lease::start(
        target,
        Opening {
            endpoints,
            admission,
        },
    ));
    let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
    }));
    drop(limit);
    assert!(
        matches!(observed, Ok(Poll::Ready(Err(_)))),
        "T1090 runtime rejection released funding before listeners"
    );
    drop(future);
    assert!(proof.is_complete());
    for address in addresses {
        drop(TcpListener::bind(address).unwrap());
    }
}

struct NoFiles(libc::rlimit);
impl NoFiles {
    fn enter() -> Self {
        let mut original = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        assert_eq!(
            unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut original) },
            0
        );
        let limited = libc::rlimit {
            rlim_cur: 0,
            rlim_max: original.rlim_max,
        };
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limited) }, 0);
        Self(original)
    }
}
impl Drop for NoFiles {
    fn drop(&mut self) {
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &self.0) }, 0);
    }
}
