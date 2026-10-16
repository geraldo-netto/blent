//! Original receiver membership and borrowed capability sources. No copied grants.
use crate::capture_contract::{LivenessCapability, Signal, ViewerActivity};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, Weak,
};
use tokio::sync::watch;

pub(super) struct Entry {
    capability: Option<Arc<dyn LivenessCapability>>,
}

pub(super) struct Registry {
    entries: Mutex<Vec<Weak<Entry>>>,
    pub(super) epoch: Arc<AtomicU64>,
    changes: watch::Sender<()>,
}

impl Registry {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::new(Vec::new()),
            epoch: Default::default(),
            changes: watch::channel(()).0,
        })
    }

    pub(super) fn add(&self, capability: Option<Arc<dyn LivenessCapability>>) -> Arc<Entry> {
        let entry = Arc::new(Entry { capability });
        self.entries.lock().unwrap().push(Arc::downgrade(&entry));
        self.changes.send_modify(|_| {});
        entry
    }

    pub(super) fn remove(
        &self,
        original: &Arc<Entry>,
        receiver: &mut Option<tokio::sync::broadcast::Receiver<crate::media::VideoPacket>>,
    ) {
        // Retire actual subscription and membership together. Observation futures
        // may retain the source, but cannot retain membership after owner drop.
        let mut entries = self.entries.lock().unwrap();
        drop(receiver.take());
        let original = Arc::downgrade(original);
        entries.retain(|entry| !Weak::ptr_eq(entry, &original));
        self.changes.send_modify(|_| {});
    }
}

impl ViewerActivity for Registry {
    fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    fn liveness_ready(&self) -> bool {
        let entries = self.entries.lock().unwrap();
        !entries.is_empty()
            && entries.iter().all(|entry| {
                entry.upgrade().is_some_and(|entry| {
                    entry
                        .capability
                        .as_ref()
                        .is_some_and(|source| source.supported())
                })
            })
    }

    fn changed(&self) -> Signal<'static> {
        // Arm structural observation before inspecting membership; arm each
        // original source before the caller reads current liveness readiness.
        let mut structural = self.changes.subscribe();
        let mut signals = vec![Box::pin(async move {
            let _ = structural.changed().await;
        }) as Signal<'static>];
        for entry in self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter_map(Weak::upgrade)
        {
            if let Some(source) = &entry.capability {
                signals.push(source.changed());
            }
        }
        Box::pin(async move {
            futures_util::future::select_all(signals).await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1041_retired_entries_are_removed_even_while_original_notices_survive() {
        let (video, initial) = crate::video_queue::channel(8, Default::default());
        drop(initial);
        for _ in 0..1000 {
            let original = video.subscribe();
            assert_eq!(video.viewers.entries.lock().unwrap().len(), 1);
            let retained_notice = video.viewers.changed();
            drop(original);
            assert_eq!(video.viewers.entries.lock().unwrap().len(), 0);
            assert_eq!(video.receiver_count(), 0);
            drop(retained_notice);
        }
    }
}
