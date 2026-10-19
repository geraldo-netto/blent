//! T1102: actual native-store exclusion through queued and withdrawn consumers.
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, SyncSender},
    Arc,
};

// A containing sponsor releases the original native call on every exit, then
// observes original task/native/authority retirement before deleting storage.
// Multithreaded test runtime keeps the original async cleanup schedulable.
struct Original {
    server: Option<Server>,
    handle: Option<link::Handle>,
    release: Option<SyncSender<()>>,
    completions: Vec<Completion>,
    _root: tempfile::TempDir,
}
impl Drop for Original {
    fn drop(&mut self) {
        drop(self.server.take());
        drop(self.handle.take());
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        for completion in &self.completions {
            while !completion.is_complete() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
fn settings(server: &Server) -> link::Settings {
    link::Settings {
        identity: server.identity.identity.clone(),
        limits: server.settings.session,
        block: server.settings.block,
        setup_timeout: Duration::from_millis(server.settings.control.setup_ms as u64),
        approval_timeout: Duration::from_millis(server.settings.control.approval_ms as u64),
        origin: server.origin,
    }
}
async fn retired(completion: &Completion) {
    bounded(async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
}
fn excluded(path: &std::path::Path) {
    let unexpected = crate::broker::linux::state::open(path, 2);
    let bypassed = unexpected.is_ok();
    drop(unexpected);
    assert!(
        !bypassed,
        "T1102 original identity consumer lost store exclusion"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t1102_withdrawn_link_keeps_native_identity_until_original_factory_returns() {
    let Fixture { server, _root } = Fixture::new().await;
    let path = server.settings.state.clone();
    let pool = Pool::new(1).unwrap();
    let (release, receive) = sync_channel(1);
    let mut original = Original {
        release: Some(release),
        completions: vec![server.authority.completion()],
        server: Some(server),
        handle: None,
        _root,
    };
    let entered = Arc::new(AtomicBool::new(false));
    let started = entered.clone();
    let server = original.server.as_ref().unwrap();
    let identity = server.identity.identity.clone();
    let certificate = identity.certificate().to_vec();
    let handle = link::Handle::start_factory(
        &pool,
        move || {
            started.store(true, Ordering::Release);
            receive.recv().unwrap();
            assert_eq!(identity.certificate(), certificate);
            anyhow::bail!("T1102 original native creator rejected")
        },
        settings(server),
    )
    .unwrap();
    original.handle = Some(handle);
    let handle = original.handle.as_ref().unwrap();
    let task = handle.task_completion();
    let native = handle.native_completion();
    original.completions.extend([task.clone(), native.clone()]);
    bounded(async {
        while !entered.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    drop(original.server.take());
    retired(&original.completions[0]).await;
    excluded(&path);
    original.handle.as_ref().unwrap().stop();
    drop(original.handle.take());
    retired(&task).await;
    assert!(!native.is_complete());
    assert_eq!(pool.active(), 1);
    excluded(&path);
    original.release.take().unwrap().send(()).unwrap();
    retired(&native).await;
    assert_eq!(pool.active(), 0);
    assert!(crate::broker::linux::state::open(&path, 2).is_ok());
}

// Runtime precedes storage destruction even if an assertion unwinds. Its normal
// drop cancels queued tasks and joins any original blocking work without timeout.
struct QueuedContext {
    authority: Completion,
    runtime: Option<tokio::runtime::Runtime>,
    _root: tempfile::TempDir,
}
impl Drop for QueuedContext {
    fn drop(&mut self) {
        drop(self.runtime.take());
        while !self.authority.is_complete() {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
#[test]
fn t1102_queued_identity_outlives_provider_before_first_task_poll() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let Fixture { server, _root } = runtime.block_on(Fixture::new());
    let context = QueuedContext {
        authority: server.authority.completion(),
        runtime: Some(runtime),
        _root,
    };
    context.runtime.as_ref().unwrap().block_on(async move {
        let path = server.settings.state.clone();
        let pool = Pool::new(1).unwrap();
        let entered = Arc::new(AtomicBool::new(false));
        let started = entered.clone();
        let handle = link::Handle::start_factory(
            &pool,
            move || {
                started.store(true, Ordering::Release);
                anyhow::bail!("T1102 queued creator must never run")
            },
            settings(&server),
        )
        .unwrap();
        // No await since admission: this task cannot have run on this executor.
        drop(server);
        assert!(!entered.load(Ordering::Acquire));
        excluded(&path);
        let task = handle.task_completion();
        let native = handle.native_completion();
        handle.stop();
        drop(handle);
        retired(&task).await;
        retired(&native).await;
        assert!(!entered.load(Ordering::Acquire));
        assert!(crate::broker::linux::state::open(&path, 2).is_ok());
    });
}
