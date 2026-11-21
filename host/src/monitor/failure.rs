//! Publish failed ADB-session liveness before awaiting native cleanup.
use super::*;
use blent::session_state::Failure;

impl Monitor {
    pub(super) fn report_failure(&self, report: &watch::Sender<Option<Failure>>) -> bool {
        let sessions =
            self.extras
                .iter()
                .chain(self.pending.iter().filter_map(|(serial, pending)| {
                    pending.session.as_ref().map(|runtime| (serial, runtime))
                }));
        for (serial, runtime) in sessions {
            if let Some(mut failure) =
                crate::daemon_supervision::session_failure(&runtime.status.borrow())
            {
                failure.message = format!("ADB tablet {serial}: {}", failure.message);
                report.send_replace(Some(failure));
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blent::session_state::State;

    #[tokio::test]
    async fn t1510_adb_failures_publish_before_session_retirement() {
        for pending in [false, true] {
            let mut monitor = super::super::tests::fixture();
            let runtime = crate::daemon_supervision::tests::failed_runtime(1);
            let mut status = runtime.status.clone();
            status
                .wait_for(|value| !matches!(value, State::Running))
                .await
                .unwrap();
            let serial = "T1510-tablet".to_string();
            if pending {
                monitor.pending.insert(
                    serial.clone(),
                    Pending {
                        instance: 1,
                        session: Some(runtime),
                    },
                );
            } else {
                monitor.extras.insert(serial.clone(), runtime);
            }
            let (report, failure) = watch::channel(None);
            let failed = monitor.report_failure(&report);
            let runtime = if pending {
                monitor.pending.remove(&serial).unwrap().session.unwrap()
            } else {
                monitor.extras.remove(&serial).unwrap()
            };
            let retired = runtime.stop().await;
            assert!(retired.is_complete());
            assert!(failed, "T1510 failed ADB session stayed healthy");
            assert!(failure.borrow().as_ref().unwrap().message.contains(&serial));
            assert!(failure
                .borrow()
                .as_ref()
                .unwrap()
                .message
                .contains("T1510 injected"));
        }
    }
}
