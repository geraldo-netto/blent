//! Bounded identity observations. Exact query incarnations fence endpoint reuse;
//! native child/collector retirement stays owned by the Commands adapter.
use crate::device_tasks::DeviceTasks;
use crate::usb::{Adb, Commands};
use futures_util::FutureExt;
use std::{collections::HashMap, sync::Arc, time::Duration};

struct Observation {
    origin: Arc<()>,
    probed: bool,
    scheduled: bool, // The task belongs to this inventory incarnation.
}
type Reply = (Arc<()>, Option<String>);

pub(super) struct Queries {
    observed: HashMap<String, Observation>,
    work: DeviceTasks<Reply>,
}
impl Queries {
    pub fn new() -> Self {
        Self {
            observed: HashMap::new(),
            work: DeviceTasks::new(4),
        }
    }
    pub fn refresh<C: Commands + Clone + 'static>(&mut self, adb: &Adb<C>, devices: &[String]) {
        for serial in self
            .observed
            .keys()
            .filter(|serial| !devices.contains(serial))
        {
            self.work.cancel(serial);
        }
        self.observed.retain(|serial, _| devices.contains(serial));
        for serial in devices {
            let observation = self
                .observed
                .entry(serial.clone())
                .or_insert_with(|| Observation {
                    origin: Arc::new(()),
                    probed: false,
                    scheduled: false,
                });
            if self.work.schedule(
                serial.clone(),
                probe(adb.clone(), serial.clone(), observation.origin.clone()),
            ) {
                observation.scheduled = true;
            }
        }
    }
    pub fn next(&mut self) -> Option<(String, Option<String>)> {
        loop {
            let (serial, result) = self.work.next().now_or_never()?;
            let Some(observation) = self.observed.get_mut(&serial) else {
                continue;
            };
            if !std::mem::take(&mut observation.scheduled) {
                continue;
            }
            let identity = match result {
                Ok((origin, identity)) if Arc::ptr_eq(&origin, &observation.origin) => identity,
                Ok(_) => continue,
                Err(_) => None,
            };
            observation.probed = true;
            return Some((serial, identity));
        }
    }
    pub fn ready(&self, serial: &str) -> bool {
        self.observed
            .get(serial)
            .is_some_and(|observation| observation.probed)
    }
    pub fn complete(&self) -> bool {
        self.observed.values().all(|observation| observation.probed)
    }
    pub async fn stop(&mut self) {
        self.work.stop().await;
        self.observed.clear();
    }
}
async fn probe<C: Commands>(adb: Adb<C>, serial: String, origin: Arc<()>) -> Reply {
    let identity = tokio::time::timeout(Duration::from_secs(5), adb.identity(&serial))
        .await
        .unwrap_or(None);
    (origin, identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usb::CommandFuture;

    #[derive(Clone)]
    struct Pending;
    impl Commands for Pending {
        fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
            Box::pin(std::future::pending())
        }
    }
    fn observation(scheduled: bool) -> Observation {
        Observation {
            origin: Arc::new(()),
            probed: false,
            scheduled,
        }
    }
    #[tokio::test(start_paused = true)]
    async fn t1381_query_deadline_publishes_unknown_and_releases_worker() {
        let mut queries = Queries::new();
        queries.refresh(&Adb(Pending), &["USB".into()]);
        tokio::task::yield_now().await;
        assert!(queries.next().is_none());
        assert!(!queries.complete());
        tokio::time::advance(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;
        assert_eq!(queries.next(), Some(("USB".into(), None)));
        assert!(queries.complete());
        assert!(!queries.work.contains("USB"));
        queries.stop().await;
        assert!(!queries.ready("USB"));
    }
    #[tokio::test]
    async fn t1381_absent_and_stale_completions_cannot_publish() {
        let mut queries = Queries::new();
        queries.observed.insert("stale".into(), observation(true));
        queries.observed.insert("reused".into(), observation(false));
        for serial in ["absent", "stale", "reused"] {
            queries.work.schedule(serial.into(), async {
                (Arc::new(()), Some("old identity".into()))
            });
        }
        tokio::task::yield_now().await;
        assert!(queries.next().is_none());
        assert!(!queries.ready("stale"));
        assert!(!queries.ready("reused"));
        queries.stop().await;
    }
    #[tokio::test]
    async fn t1381_failed_task_only_invalidates_its_current_observation() {
        let mut queries = Queries::new();
        queries.observed.insert("current".into(), observation(true));
        queries.observed.insert("reused".into(), observation(false));
        for serial in ["current", "reused"] {
            queries.work.schedule(serial.into(), async {
                panic!("T1381 injected query task failure");
            });
        }
        tokio::task::yield_now().await;
        assert_eq!(queries.next(), Some(("current".into(), None)));
        assert!(queries.next().is_none());
        assert!(queries.ready("current"));
        assert!(!queries.ready("reused"));
        queries.stop().await;
    }
}
