use blent::{
    session::Runtime,
    usb::{Adb, Routes},
};
use blent_config::retirement::Completion;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
#[path = "routes.rs"]
mod routes;
#[path = "session.rs"]
mod session;

#[derive(Default)]
pub(super) struct State {
    release: AtomicBool,
    native_done: AtomicBool,
    route_lease_released: AtomicBool,
    wake: tokio::sync::Notify,
}
struct Owned {
    _session: Runtime,
    _routes: Routes,
    _shutdown: tokio::sync::watch::Sender<bool>,
}
struct Proofs {
    session: Completion,
    routes: Completion,
}
async fn create(state: Arc<State>) -> (Owned, Proofs, routes::Fake) {
    let (session, shutdown) = session::create(state.clone()).await;
    let adb = Adb(routes::Fake::new(state));
    let mut routes = Routes::new("T1411USB", (19000, 19001)).unwrap();
    routes.prepare(&adb).await.unwrap();
    let proofs = Proofs {
        session: session.retirement.clone(),
        routes: routes.completion().unwrap(),
    };
    (
        Owned {
            _session: session,
            _routes: routes,
            _shutdown: shutdown,
        },
        proofs,
        adb.0,
    )
}
async fn hold(owned: Owned, ready: tokio::sync::oneshot::Sender<()>) {
    ready.send(()).unwrap();
    std::future::pending::<()>().await;
    drop(owned);
}
async fn work(
    mode: String,
    state: Arc<State>,
    ready: mpsc::Sender<(Proofs, routes::Fake)>,
) -> Result<(), &'static str> {
    let (owned, proofs, fake) = create(state).await;
    let (started, entered) = tokio::sync::oneshot::channel();
    let holder = tokio::spawn(hold(owned, started));
    entered.await.unwrap();
    if mode == "cancel" {
        holder.abort();
        let _ = holder.await;
    }
    ready.send((proofs, fake)).unwrap();
    match mode.as_str() {
        "error" => Err("T1411 work error"),
        "panic" => panic!("T1411 work panic"),
        _ => Ok(()),
    }
}
pub(super) fn run(mode: &str) {
    let state = Arc::new(State::default());
    let (ready, staged) = mpsc::channel();
    let (done, returned) = mpsc::channel();
    let owned = state.clone();
    let selected = mode.to_owned();
    let worker = std::thread::spawn(move || {
        let result =
            std::panic::catch_unwind(|| blent::runtime_owner::run(work(selected, owned, ready)));
        done.send(result).unwrap();
    });
    let (proofs, fake) = staged.recv_timeout(Duration::from_secs(5)).unwrap();
    let premature = returned.recv_timeout(Duration::from_millis(200));
    let held = !state.route_lease_released.load(Ordering::Acquire);
    state.release.store(true, Ordering::Release);
    state.wake.notify_one();
    let was_early = premature.is_ok();
    let result = premature.unwrap_or_else(|_| {
        returned
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|error| {
                panic!(
                    "T1411 drain stalled: {error}; native={}, session={}, routes={}, mappings={}",
                    state.native_done.load(Ordering::Acquire),
                    proofs.session.is_complete(),
                    proofs.routes.is_complete(),
                    fake.count()
                )
            })
    });
    worker.join().unwrap();
    assert!(
        !was_early,
        "T1411 process entry returned before original cleanup settled"
    );
    assert!(held, "T1411 original route lease released before cleanup");
    assert!(
        state.native_done.load(Ordering::Acquire),
        "T1411 async native cleanup lost its runtime"
    );
    assert!(state.route_lease_released.load(Ordering::Acquire));
    assert!(proofs.session.is_complete() && proofs.routes.is_complete());
    assert_eq!(fake.count(), 0);
    check_outcome(mode, result);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut routes = Routes::new("T1411USB", (19000, 19001)).unwrap();
        assert!(
            routes.prepare(&Adb(fake)).await.is_err(),
            "T1411 route admission reopened"
        );
    });
    assert!(
        blent::session::Workers::reserve().is_err(),
        "T1411 process admission reopened"
    );
}
fn check_outcome(
    mode: &str,
    result: std::thread::Result<std::io::Result<Result<(), &'static str>>>,
) {
    match mode {
        "panic" => assert!(result.is_err()),
        "error" => assert_eq!(result.unwrap().unwrap(), Err("T1411 work error")),
        _ => assert_eq!(result.unwrap().unwrap(), Ok(())),
    }
}
