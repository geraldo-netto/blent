//! Linux signal ownership begins before startup can admit resources.
use anyhow::Result;
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::watch,
    task::JoinHandle,
};

/// The entry owns these guards outside runtime_owner::run, including its drain.
#[derive(Default)]
pub(crate) struct Lifetime {
    pub(crate) publication: Option<crate::daemon_pid::Publication>,
    pub(crate) signals: Option<Listener>,
}

pub(crate) struct Listener {
    requested: watch::Receiver<bool>,
    task: JoinHandle<()>,
}
impl Listener {
    pub(crate) fn new() -> std::io::Result<Self> {
        // Registration is eager, before the spawned observer is first polled.
        let mut terminate = signal(SignalKind::terminate())?;
        let mut interrupt = signal(SignalKind::interrupt())?;
        let (request, requested) = watch::channel(false);
        let task = tokio::spawn(async move {
            loop {
                let event = tokio::select! {
                    event = terminate.recv() => event,
                    event = interrupt.recv() => event,
                };
                if event.is_none() {
                    break;
                }
                // Publish before cutoff; repeated signals remain observable.
                let repeated = request.send_replace(true);
                if repeated {
                    tracing::warn!(
                        "Termination requested again; waiting for original native retirement"
                    );
                } else {
                    blent::runtime_owner::request_shutdown();
                }
            }
        });
        Ok(Self { requested, task })
    }

    /// Cancel an owned command on a catchable signal; entry teardown retains cleanup.
    pub(crate) async fn cancel_on_signal(
        &self,
        work: impl std::future::Future<Output = Result<()>>,
    ) -> Result<()> {
        let result = tokio::select! {
            result = work => result,
            _ = crate::wait_for_shutdown(self.requested.clone()) => {
                blent::runtime_owner::request_shutdown();
                Ok(())
            },
        };
        self.finish(result)?;
        anyhow::ensure!(!*self.requested.borrow(), "Command interrupted");
        Ok(())
    }

    pub(crate) async fn wait(&self, quit: watch::Receiver<bool>) -> Result<()> {
        tokio::select! {
            _ = crate::wait_for_shutdown(self.requested.clone()) => {},
            _ = crate::wait_for_shutdown(quit) => {},
        }
        // A remembered request also guarantees cutoff before the caller proceeds.
        blent::runtime_owner::request_shutdown();
        Ok(())
    }

    pub(crate) fn finish(&self, result: Result<()>) -> Result<()> {
        result.or_else(|error| {
            if *self.requested.borrow() && error.chain().any(admission_closed) {
                Ok(())
            } else {
                Err(error)
            }
        })
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        // The observer owns no native lease; dropping signals deregisters its streams.
        self.task.abort();
    }
}

fn admission_closed(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .and_then(std::io::Error::get_ref)
        .is_some_and(|cause| cause.is::<blent::runtime_owner::AdmissionClosed>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closed() -> anyhow::Error {
        std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            blent::runtime_owner::AdmissionClosed,
        )
        .into()
    }

    #[tokio::test]
    async fn t1423_only_requested_admission_rejection_becomes_graceful_stop() {
        let mut listener = Listener::new().unwrap();
        assert!(listener.finish(Ok(())).is_ok());
        assert!(listener.finish(Err(closed())).is_err());
        let (_request, received) = watch::channel(true);
        listener.requested = received;
        assert!(listener.finish(Err(closed().context("startup"))).is_ok());
        for error in [
            anyhow::anyhow!("invalid configuration"),
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "unrelated I/O").into(),
        ] {
            let expected = error.to_string();
            assert_eq!(
                listener.finish(Err(error)).unwrap_err().to_string(),
                expected
            );
        }
        assert_eq!(
            blent::runtime_owner::AdmissionClosed.to_string(),
            "Process resource admission is closed"
        );
    }

    #[tokio::test]
    async fn t1509_repeated_signal_remains_observable_during_drain() {
        if crate::daemon_activity::tests::isolated_daemon(
            "daemon_shutdown::tests::t1509_repeated_signal_remains_observable_during_drain",
        ) {
            return;
        }
        let listener = Listener::new().unwrap();
        let mut requests = listener.requested.clone();
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGTERM] {
            assert_eq!(unsafe { libc::kill(libc::getpid(), signal) }, 0);
            let result =
                tokio::time::timeout(std::time::Duration::from_secs(3), requests.changed()).await;
            assert!(
                matches!(result, Ok(Ok(()))),
                "T1509 later termination signal was swallowed: {result:?}"
            );
            assert!(*requests.borrow_and_update());
        }
        drop(listener);
    }

    #[tokio::test]
    async fn t1423_owner_drop_aborts_original_signal_observer() {
        let listener = Listener::new().unwrap();
        let observer = listener.task.abort_handle();
        drop(listener);
        tokio::task::yield_now().await;
        assert!(observer.is_finished());
    }
}
