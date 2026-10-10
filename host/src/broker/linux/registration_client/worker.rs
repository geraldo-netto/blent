use super::*;
use crate::broker::linux::{async_io::AsyncSocket, Received};
use blent_config::connection::registration::Message;
use std::future::Future;

pub(super) struct Worker {
    settings: Settings,
    connector: Arc<Connector>,
    shared: Arc<Shared>,
    endpoints: Arc<Endpoints>,
}
impl Worker {
    pub(super) fn new(
        settings: Settings,
        connector: Arc<Connector>,
        shared: Arc<Shared>,
        endpoints: Arc<Endpoints>,
    ) -> Self {
        Self {
            settings,
            connector,
            shared,
            endpoints,
        }
    }
    pub(super) fn run(self) {
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => self.finish_runtime(runtime, self.drive()),
            Err(_) => self.shared.fail(Failure::Native),
        }
    }
    fn finish_runtime(
        &self,
        runtime: tokio::runtime::Runtime,
        operation: impl Future<Output = Result<(), Failure>>,
    ) {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(operation)))
                .unwrap_or(Err(Failure::Native));
        if let Err(error) = result {
            self.shared.fail(error);
        }
        // Retained thread waits for any native Connector starter operation here.
        // Neither UI nor the shared retirement reaper blocks on runtime teardown.
        drop(runtime);
    }
    async fn drive(&self) -> Result<(), Failure> {
        let socket = tokio::select! { biased;
            _ = self.shared.wake.notified() => return Ok(()),
            result = self.connector.connect(self.settings.startup) => result.map_err(|_| Failure::Native)?,
        };
        self.exchange(&socket).await
    }
    async fn exchange(&self, socket: &AsyncSocket) -> Result<(), Failure> {
        let admission = Instant::now() + self.settings.admission;
        while !self.shared.state().model.status().remote_retired {
            let message = self.shared.update(Model::outgoing);
            if let Some(message) = message {
                self.send(socket, &message, admission).await?;
            }
            if let Some(packet) = self.wait(admission, true, socket.receive()).await? {
                self.receive(packet);
            }
        }
        Ok(())
    }
    async fn send(
        &self,
        socket: &AsyncSocket,
        message: &Message,
        admission: Instant,
    ) -> Result<(), Failure> {
        let descriptors = if matches!(message, Message::Register { .. }) {
            self.endpoints.descriptors()
        } else {
            Vec::new()
        };
        self.wait(admission, false, socket.send(message, &descriptors))
            .await
            .map(|_| ())
    }
    fn receive(&self, packet: Received) {
        if !packet.descriptors.is_empty() {
            self.shared.fail(Failure::Protocol);
            return;
        }
        // Malformed replies revoke admission, but retain the IPC cleanup path.
        // A later exact Retired may still establish remote retirement.
        let _ = self.shared.update(|model| model.receive(packet.message));
    }
    async fn wait<T>(
        &self,
        admission: Instant,
        interruptible: bool,
        operation: impl Future<Output = std::io::Result<T>>,
    ) -> Result<Option<T>, Failure> {
        tokio::pin!(operation);
        loop {
            let deadline = self.shared.deadline(admission, self.settings.cleanup);
            tokio::select! { biased;
                _ = wait_deadline(deadline) => return Err(Failure::TimedOut),
                result = &mut operation => return result.map(Some).map_err(|_| Failure::Native),
                _ = self.shared.wake.notified() => { if interruptible { return Ok(None); } },
            }
        }
    }
}
async fn wait_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn worker() -> Worker {
        let settings = Settings::new(Launch {
            executable: "/bin/false".into(),
            runtime: "/tmp".into(),
            state: "/tmp".into(),
            helper: "/bin/false".into(),
        });
        let request = super::super::tests::request();
        let endpoints = Endpoints::validate(
            request.resource,
            super::super::tests::listeners()
                .into_iter()
                .map(OwnedFd::from)
                .collect(),
        )
        .unwrap();
        Worker::new(
            settings.clone(),
            Arc::new(Connector::new(settings.launch.clone(), 1).unwrap()),
            Arc::new(Shared::new(Model::new(request).unwrap())),
            Arc::new(endpoints),
        )
    }
    #[test]
    fn t911_panic_revokes_before_blocked_native_runtime_cleanup_exits() {
        let worker = worker();
        let shared = worker.shared.clone();
        let (release, wait) = std::sync::mpsc::channel();
        let (entered, entry) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.spawn_blocking(move || {
                entered.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(3)).unwrap();
            });
            worker.finish_runtime(runtime, async { panic!("T911 fixture panic") });
        });
        entry.recv_timeout(Duration::from_secs(3)).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while shared.state().model.status().failure.is_none() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(shared.state().model.status().failure, Some(Failure::Native));
        assert!(!thread.is_finished());
        release.send(()).unwrap();
        thread.join().unwrap();
    }
    #[tokio::test(start_paused = true)]
    async fn t911_wake_cannot_extend_a_blocked_send_cleanup_deadline() {
        let worker = worker();
        worker.shared.update(Model::outgoing);
        worker.shared.update(Model::close);
        let admission = Instant::now() + Duration::from_secs(30);
        worker.shared.wake.notify_one();
        let result = worker
            .wait::<()>(admission, false, std::future::pending())
            .await;
        assert_eq!(result, Err(Failure::TimedOut));
    }
}
