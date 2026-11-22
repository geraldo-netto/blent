//! T1716: a per-fixture barrier in the actual blocking preparation thread.
//! Dropping the gate removes future admission and releases the original waiter.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex, OnceLock},
};

struct Held {
    entered: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: Mutex<mpsc::Receiver<()>>,
}
fn gates() -> &'static Mutex<HashMap<PathBuf, Arc<Held>>> {
    static GATES: OnceLock<Mutex<HashMap<PathBuf, Arc<Held>>>> = OnceLock::new();
    GATES.get_or_init(Default::default)
}
// The two preparation regressions can each admit one original barrier.
const MAX_GATES: usize = 2;

fn admit(path: &Path, original: &Arc<Held>) -> bool {
    let mut entries = gates().lock().unwrap();
    if entries.len() >= MAX_GATES || entries.contains_key(path) {
        return false;
    }
    entries.insert(path.to_owned(), original.clone());
    true
}

pub(crate) struct Gate {
    path: PathBuf,
    original: Arc<Held>,
    entered: Option<tokio::sync::oneshot::Receiver<()>>,
    release: Option<mpsc::Sender<()>>,
}
impl Gate {
    pub(crate) fn new(path: PathBuf) -> Self {
        let (started, entered) = tokio::sync::oneshot::channel();
        let (release, waiter) = mpsc::channel();
        let original = Arc::new(Held {
            entered: Mutex::new(Some(started)),
            release: Mutex::new(waiter),
        });
        assert!(
            admit(&path, &original),
            "T1716 duplicate identity or full fixture capacity"
        );
        Self {
            path,
            original,
            entered: Some(entered),
            release: Some(release),
        }
    }
    pub(crate) async fn entered(&mut self) {
        self.entered.take().unwrap().await.unwrap();
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        let mut entries = gates().lock().unwrap();
        if entries
            .get(&self.path)
            .is_some_and(|entry| Arc::ptr_eq(entry, &self.original))
        {
            entries.remove(&self.path);
        }
        drop(self.release.take());
    }
}
pub(super) fn wait(path: &Path) {
    let original = gates().lock().unwrap().get(path).cloned();
    if let Some(original) = original {
        if let Some(entered) = original.entered.lock().unwrap().take() {
            let _ = entered.send(());
        }
        let _ = original.release.lock().unwrap().recv();
    }
}
