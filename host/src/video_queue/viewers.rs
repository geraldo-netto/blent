//! Original receiver membership and borrowed capability sources. No copied grants.
use crate::capture_contract::{LivenessCapability, Signal, ViewerActivity, ViewerPresence};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, Weak,
};
use tokio::sync::watch;

pub(super) struct Entry {
    capability: Option<Arc<dyn LivenessCapability>>,
}

pub(super) struct Registry {
    members: Mutex<Members>,
    pub(super) epoch: Arc<AtomicU64>,
    changes: watch::Sender<()>,
}

#[derive(Default)]
struct Members {
    entries: Vec<Weak<Entry>>,
    continuity: Arc<()>,
}

impl Registry {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            members: Mutex::new(Members::default()),
            epoch: Default::default(),
            changes: watch::channel(()).0,
        })
    }

    pub(super) fn add(&self, capability: Option<Arc<dyn LivenessCapability>>) -> Arc<Entry> {
        let entry = Arc::new(Entry { capability });
        let mut members = self.members.lock().unwrap();
        if members.entries.is_empty() {
            members.continuity = Arc::new(());
        }
        members.entries.push(Arc::downgrade(&entry));
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
        let mut members = self.members.lock().unwrap();
        drop(receiver.take());
        let original = Arc::downgrade(original);
        members
            .entries
            .retain(|entry| !Weak::ptr_eq(entry, &original));
        self.changes.send_modify(|_| {});
    }
}

impl ViewerActivity for Registry {
    fn presence(&self) -> Option<ViewerPresence> {
        let members = self.members.lock().unwrap();
        Some(ViewerPresence::new(
            members.entries.len(),
            members.continuity.clone(),
        ))
    }

    fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    fn liveness_ready(&self) -> bool {
        let members = self.members.lock().unwrap();
        !members.entries.is_empty()
            && members.entries.iter().all(|entry| {
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
            .members
            .lock()
            .unwrap()
            .entries
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
            let original = video.subscribe().unwrap();
            assert_eq!(video.viewers.members.lock().unwrap().entries.len(), 1);
            let retained_notice = video.viewers.changed();
            drop(original);
            assert_eq!(video.viewers.members.lock().unwrap().entries.len(), 0);
            assert_eq!(video.receiver_count(), 0);
            drop(retained_notice);
        }
    }
}

#[cfg(test)]
mod exhaustion_tests;
