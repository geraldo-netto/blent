//! T1101: configured production workflow observes one original held completion.
use super::*;

struct Original {
    server: Option<Server>,
    authority: Completion,
    native: Option<Completion>,
    release: Release,
    _root: tempfile::TempDir,
}
impl Original {
    fn new(fixture: Fixture) -> Self {
        let Fixture { server, _root } = fixture;
        Self {
            authority: server.authority.completion(),
            server: Some(server),
            native: None,
            release: Release(Arc::new(AtomicBool::new(false))),
            _root,
        }
    }
    fn hold(&mut self) -> Pool {
        let server = self.server.as_mut().unwrap();
        // Isolate deadline observation from inventory subprocess scheduling.
        server.next_inventory = Instant::now() + Duration::from_secs(3600);
        let client = server.controller.admit_client().unwrap();
        let id = tls::selection(server, client);
        let admission = server.link_pool.reserve().unwrap();
        let completion = admission.completion();
        self.native = Some(completion.clone());
        admission.retain(Held(self.release.0.clone()));
        server.controller.attach(id, completion).unwrap();
        server.links.insert(
            id,
            Link {
                handle: None,
                completion: completed_task(),
                verified: None,
            },
        );
        server.link_pool.clone()
    }
}
impl Drop for Original {
    fn drop(&mut self) {
        drop(self.server.take());
        self.release.0.store(true, Ordering::Release);
        for completion in std::iter::once(&self.authority).chain(self.native.iter()) {
            while !completion.is_complete() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
async fn configured_budget(milliseconds: u64) {
    let fixture = Fixture::configured(|settings| {
        settings.retirement_timeout = Duration::from_millis(milliseconds);
        settings.control.links = 1;
    })
    .await;
    let mut original = Original::new(fixture);
    let pool = original.hold();
    tokio::time::pause();
    // Independent requested timer is the oracle for the installed runtime's
    // millisecond rounding. Never await work after this timer: a fixed five-
    // second regression must fail immediately at the selected timer boundary.
    let mut expected = Box::pin(tokio::time::sleep(Duration::from_millis(milliseconds)));
    assert!(futures_util::poll!(&mut expected).is_pending());
    let mut work = Box::pin(crate::broker::linux::entry::serve_opened(
        original.server.take().unwrap(),
        async { Ok(()) },
    ));
    assert!(futures_util::poll!(&mut work).is_pending());
    tokio::time::advance(Duration::from_millis(milliseconds - 1)).await;
    assert!(futures_util::poll!(&mut work).is_pending());
    tokio::time::advance(Duration::from_millis(1)).await;
    expected.await;
    let std::task::Poll::Ready(result) = futures_util::poll!(&mut work) else {
        panic!("T1101 entry ignored selected cleanup observation deadline");
    };
    let error = result.unwrap_err();
    assert!(error
        .downcast_ref::<tokio::time::error::Elapsed>()
        .is_some());
    assert!(!original.native.as_ref().unwrap().is_complete());
    assert_eq!(pool.active(), 1);
    assert!(pool.reserve().is_err());
    tokio::time::resume();
    drop(original);
    assert_eq!(pool.active(), 0);
    assert!(pool.reserve().is_ok());
}
#[tokio::test]
async fn t1101_entry_uses_selected_budget_without_retiring_original_native_obligation() {
    for milliseconds in [1, 37, 60_000] {
        configured_budget(milliseconds).await;
    }
}
