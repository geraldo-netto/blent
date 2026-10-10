use super::*;
use blent_config::connection::{control::Command, control_view::Model};

pub(super) struct Worker {
    settings: Settings,
    connector: Arc<Connector>,
    shared: Arc<Shared>,
}
impl Worker {
    pub(super) fn new(settings: Settings, connector: Arc<Connector>, shared: Arc<Shared>) -> Self {
        Self {
            settings,
            connector,
            shared,
        }
    }
    pub(super) fn run(self, commands: mpsc::Receiver<Request>, stop: oneshot::Receiver<()>) {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                self.shared.finish(Err(error.into()));
                return;
            }
        };
        self.finish_runtime(runtime, async {
            tokio::select! { biased;
                _ = stop => Ok(()),
                result = self.drive(commands) => result,
            }
        });
    }
    fn finish_runtime(
        &self,
        runtime: tokio::runtime::Runtime,
        operation: impl std::future::Future<Output = Result<()>>,
    ) {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(operation)))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("broker client worker panicked")));
        self.shared.finish(result);
        // Runtime Drop waits for any outstanding native starter operation. It is
        // deliberately on this retained thread, never the UI or shared reaper.
        drop(runtime);
    }
    async fn drive(&self, mut commands: mpsc::Receiver<Request>) -> Result<()> {
        let socket = self.connector.connect(self.settings.startup).await?;
        let mut model = Model::default();
        let mut request = 0;
        let mut work = (Command::Inspect, None);
        loop {
            self.shared.publish(Phase::Busy, model.view(), None, None);
            let reply = exchange::exchange(
                &socket,
                exchange::next_request(&mut request)?,
                work.0,
                self.settings.limits,
                self.settings.transaction,
            )
            .await?;
            self.apply(&mut model, reply, work.1)?;
            work = self.next(&model, &mut commands).await?;
        }
    }
    fn apply(
        &self,
        model: &mut Model,
        reply: exchange::Reply,
        completed: Option<u64>,
    ) -> Result<()> {
        match reply {
            exchange::Reply::Snapshot(snapshot) => {
                let view = model.publish(snapshot)?;
                self.shared
                    .publish(Phase::Ready, Some(view), None, completed);
            }
            exchange::Reply::Rejected(reason) => {
                model.reset();
                self.shared.publish(
                    Phase::Ready,
                    None,
                    Some(format!("Broker rejected control request: {reason:?}")),
                    completed,
                );
            }
        }
        Ok(())
    }
    async fn next(
        &self,
        model: &Model,
        commands: &mut mpsc::Receiver<Request>,
    ) -> Result<(Command, Option<u64>)> {
        let deadline = tokio::time::Instant::now() + self.settings.poll;
        loop {
            let action = tokio::select! {
                value = commands.recv() => value.ok_or_else(|| anyhow::anyhow!("broker client control closed"))?,
                _ = tokio::time::sleep_until(deadline) => return Ok((Command::Inspect, None)),
            };
            match model.command(&action.action) {
                Ok(command) => return Ok((command.clone(), Some(action.id))),
                Err(error) => self.shared.publish(
                    Phase::Ready,
                    model.view(),
                    Some(error.to_string()),
                    Some(action.id),
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn t901_stale_commands_cannot_extend_the_snapshot_refresh_deadline() {
        let mut settings = Settings::new(Launch {
            executable: "/bin/false".into(),
            runtime: "/tmp".into(),
            state: "/tmp".into(),
            helper: "/bin/false".into(),
        });
        settings.poll = Duration::from_millis(5);
        let worker = Worker::new(
            settings.clone(),
            Arc::new(Connector::new(settings.launch.clone(), 1).unwrap()),
            Arc::new(Shared::new()),
        );
        let model = Model::default();
        let (sender, mut receiver) = mpsc::channel(1);
        let stale = super::super::tests::view()
            .prepare(Command::Inspect)
            .unwrap();
        let next = worker.next(&model, &mut receiver);
        tokio::pin!(next);
        let mut observed = None;
        for id in 1..=10 {
            sender
                .send(Request {
                    id,
                    action: stale.clone(),
                })
                .await
                .unwrap();
            tokio::select! {
                result = &mut next => { observed = Some(result.unwrap()); break; },
                _ = tokio::time::sleep(Duration::from_millis(1)) => (),
            }
        }
        assert_eq!(
            observed,
            Some((Command::Inspect, None)),
            "T901 stale UI traffic postponed status refresh"
        );
    }
    #[test]
    fn t901_panic_is_reported_before_blocked_native_runtime_cleanup_finishes() {
        let settings = Settings::new(Launch {
            executable: "/bin/false".into(),
            runtime: "/tmp".into(),
            state: "/tmp".into(),
            helper: "/bin/false".into(),
        });
        let shared = Arc::new(Shared::new());
        let worker = Worker::new(
            settings.clone(),
            Arc::new(Connector::new(settings.launch.clone(), 1).unwrap()),
            shared.clone(),
        );
        let (release, wait) = std::sync::mpsc::channel();
        let (entered, entry) = std::sync::mpsc::channel();
        let (completed, completion) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.spawn_blocking(move || {
                entered.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(3)).unwrap();
            });
            worker.finish_runtime(runtime, async { panic!("T901 injected client panic") });
            completed.send(()).unwrap();
        });
        entry.recv_timeout(Duration::from_secs(3)).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while shared.state.lock().unwrap().phase != Phase::Failed {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            completion.try_recv().is_err(),
            "runtime teardown must retain blocked native work"
        );
        assert!(shared
            .state
            .lock()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .contains("panicked"));
        release.send(()).unwrap();
        completion.recv_timeout(Duration::from_secs(3)).unwrap();
        thread.join().unwrap();
    }
}

#[cfg(test)]
mod native_failure_tests {
    use super::*;
    use blent_config::commands::SyncCommandExt;

    #[test]
    fn t901_runtime_creation_failure_is_contained_in_an_isolated_process() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "broker::linux::presentation::worker::native_failure_tests::t901_runtime_failure_fixture", "--nocapture"])
            .env("BLENT_T901_RUNTIME_FAILURE", "1")
            .output_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    }

    #[test]
    fn t901_runtime_failure_fixture() {
        if std::env::var_os("BLENT_T901_RUNTIME_FAILURE").is_none() {
            return;
        }
        let settings = Settings::new(Launch {
            executable: "/bin/false".into(),
            runtime: "/tmp".into(),
            state: "/tmp".into(),
            helper: "/bin/false".into(),
        });
        let shared = Arc::new(Shared::new());
        let worker = Worker::new(
            settings.clone(),
            Arc::new(Connector::new(settings.launch.clone(), 1).unwrap()),
            shared.clone(),
        );
        let (_send, receive) = mpsc::channel(1);
        let (stop, stopped) = oneshot::channel();
        drop(stop);
        let mut original = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        assert_eq!(
            unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut original) },
            0
        );
        let limited = libc::rlimit {
            rlim_cur: original.rlim_cur.min(64),
            rlim_max: original.rlim_max,
        };
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limited) }, 0);
        let mut files = Vec::new();
        for _ in 0..64 {
            match std::fs::File::open("/dev/null") {
                Ok(file) => files.push(file),
                Err(_) => break,
            }
        }
        assert_eq!(
            std::fs::File::open("/dev/null").unwrap_err().raw_os_error(),
            Some(libc::EMFILE)
        );
        worker.run(receive, stopped);
        drop(files);
        assert_eq!(
            unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &original) },
            0
        );
        assert_eq!(shared.state.lock().unwrap().phase, Phase::Failed);
        assert!(shared.state.lock().unwrap().view.is_none());
        assert!(shared
            .state
            .lock()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .contains("Too many open files"));
    }
}
