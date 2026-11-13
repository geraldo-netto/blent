//! The process-owned reusable configuration executor must retire despite static storage.
use blent::{
    config_jobs::Store,
    persistence_port::{AsyncSnapshots, AsyncTransactions},
};
use blent_config::{
    model::FileConfig,
    persistence_contract::{Edit, Snapshots, Transactions},
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};

struct Memory {
    calls: Arc<AtomicUsize>,
    original: Mutex<Option<std::thread::ThreadId>>,
    held: Mutex<Option<super::Gate>>,
    tls: bool,
}
impl Snapshots for Memory {
    fn snapshot(&self) -> FileConfig {
        let thread = std::thread::current().id();
        let mut original = self.original.lock().unwrap();
        assert_eq!(
            *original.get_or_insert(thread),
            thread,
            "T1432 reusable shared worker"
        );
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(gate) = self.held.lock().unwrap().take() {
            super::native(gate, self.tls);
        }
        FileConfig::default()
    }
}
impl Transactions for Memory {
    fn transact(&self, _: &AtomicBool, edit: Edit<'_>) -> anyhow::Result<FileConfig> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        let mut value = FileConfig::default();
        edit(&mut value)?;
        Ok(value)
    }
}
fn memory(calls: Arc<AtomicUsize>, held: Option<super::Gate>, tls: bool) -> Memory {
    Memory {
        calls,
        original: Mutex::new(None),
        held: Mutex::new(held),
        tls,
    }
}

fn result(mode: &str) -> Result<(), &'static str> {
    match mode {
        "panic" => panic!("T1432 shared configuration exit"),
        "error" => Err("T1432: workflow error"),
        _ => Ok(()),
    }
}

pub fn run(mode: String) {
    if mode == "invalid" {
        invalid();
        return;
    }
    if mode == "queue" {
        queue();
        return;
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let (entered, entry) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let (sent, received) = mpsc::channel();
    let (done, returned) = mpsc::channel();
    let original_calls = calls.clone();
    let tls = mode == "tls";
    let original_mode = mode.clone();
    let original = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(|| {
            blent::runtime_owner::run(async {
                let gate = tls.then_some((entered, held));
                let store = Arc::new(Store::new(memory(original_calls, gate, tls)).unwrap());
                assert!(store.snapshot().await.is_ok());
                assert!(store.snapshot().await.is_ok());
                sent.send(store).unwrap();
                result(&original_mode)
            })
            .unwrap()
        });
        super::assert_outcome(outcome, &original_mode);
        done.send(()).unwrap();
    });
    let store = received.recv().unwrap();
    let premature = observe_exit(tls, entry, &returned);
    release.send(()).ok();
    original.join().unwrap();
    assert!(
        !premature,
        "T1432 shared worker TLS outlived process runtime"
    );
    assert!(
        Store::new(memory(calls.clone(), None, false)).is_err(),
        "T1432 shared executor reopened after cutoff"
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(
        runtime.block_on(store.snapshot()).is_err(),
        "T1432 retained Store admitted after cutoff"
    );
    assert_eq!(calls.load(Ordering::Acquire), 2);
}

fn observe_exit(tls: bool, entered: mpsc::Receiver<()>, returned: &mpsc::Receiver<()>) -> bool {
    if !tls {
        returned.recv().unwrap();
        return false;
    }
    // Failure still releases the gate in run before reporting the assertion.
    if entered
        .recv_timeout(std::time::Duration::from_secs(2))
        .is_err()
    {
        return true;
    }
    returned
        .recv_timeout(std::time::Duration::from_millis(100))
        .is_ok()
}

fn queue() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let (entered, entry) = mpsc::channel();
    let (release, held) = mpsc::channel();
    blent::runtime_owner::run(async {
        let store = Store::new(memory(calls, Some((entered, held)), false)).unwrap();
        let first = store.snapshot();
        tokio::pin!(first);
        assert!(futures_util::poll!(&mut first).is_pending());
        entry.recv().unwrap();
        let second = store.transact(Box::new(|_| Ok(())));
        tokio::pin!(second);
        assert!(futures_util::poll!(&mut second).is_pending());
        blent::runtime_owner::request_shutdown();
        release.send(()).unwrap();
        assert!(
            first.await.is_ok(),
            "T1432 already-started snapshot lost its outcome"
        );
        assert!(
            second.await.is_err(),
            "T1432 queued transaction ran after cutoff"
        );
    })
    .unwrap();
    assert_eq!(observed.load(Ordering::Acquire), 1);
}

fn invalid() {
    let calls = Arc::new(AtomicUsize::new(0));
    blent::runtime_owner::run(async {
        for _ in 0..2 {
            assert!(Store::new(memory(calls.clone(), None, false)).is_err());
        }
    })
    .unwrap();
    assert_eq!(calls.load(Ordering::Acquire), 0);
}
