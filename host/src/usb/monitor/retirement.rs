//! One native retirement record per slot; caller cancellation cannot release it.
use super::*;
use blent_config::retirement::Completion;

pub(super) struct RetiringSession {
    pub(super) index: usize,
    serial: Option<String>,
    identity: Option<String>,
    routes: Option<Routes>,
    proof: Completion,
}

impl RetiringSession {
    pub(super) fn matches(&self, serial: &str, identity: Option<&String>) -> bool {
        self.serial.as_deref() == Some(serial)
            || self
                .identity
                .as_ref()
                .is_some_and(|original| Some(original) == identity)
    }
}
impl<C: Commands + Clone + 'static> Monitor<C> {
    pub(super) async fn retire(&mut self, index: usize) {
        if let Some(mut active) = self.slots[index].take() {
            // Install the reservation before stop's first await. Runtime::drop
            // still sponsors its supervisor if this caller is cancelled.
            self.retiring.push(RetiringSession {
                index,
                serial: active.connection.selected(),
                identity: active.identity,
                routes: active.connection.release_routes(),
                proof: active.runtime.retirement.clone(),
            });
            active.runtime.stop().await;
            self.settle_sessions();
            self.retry_routes(None).await;
        }
    }
    pub(super) fn settle_sessions(&mut self) {
        let mut index = 0;
        while index < self.retiring.len() {
            if self.retiring[index].proof.is_complete() {
                let retired = self.retiring.swap_remove(index);
                if let Some(routes) = retired.routes {
                    self.pending.push((routes, retired.identity));
                }
            } else {
                index += 1;
            }
        }
    }
    pub(super) async fn drain_sessions(&mut self) {
        self.settle_sessions();
        while !self.retiring.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
            self.settle_sessions();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1410_original_physical_identity_blocks_alias_reuse() {
        let native = blent_config::retirement::Pool::new(1)
            .unwrap()
            .reserve()
            .unwrap();
        let mut retiring = RetiringSession {
            index: 0,
            serial: Some("wifi:5555".into()),
            identity: Some("tablet".into()),
            routes: None,
            proof: native.completion(),
        };
        assert!(retiring.matches("wifi:5555", None));
        assert!(retiring.matches("USB", Some(&"tablet".into())));
        assert!(!retiring.matches("USB", Some(&"different".into())));
        assert!(!retiring.matches("USB", None));
        retiring.identity = None;
        assert!(!retiring.matches("USB", None));
        assert!(retiring.matches("wifi:5555", None));
    }
}
