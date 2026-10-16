//! Immutable view of one original native-retirement reporting lifetime.
use crate::capture_contract::NativeRetirement;
use blent_config::retirement::Completion;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct Scope(Option<Arc<dyn NativeRetirement>>);
impl Scope {
    pub fn new(retirement: Option<Arc<dyn NativeRetirement>>) -> Self {
        Self(retirement)
    }
    pub fn retain(&self, original: Completion) {
        if let Some(retirement) = &self.0 {
            retirement.retain(original);
        }
    }
    /// Start a native child and publish its exact proof before returning it.
    pub fn spawn(
        &self,
        command: &mut tokio::process::Command,
    ) -> std::io::Result<blent_config::commands::OwnedChild> {
        Ok(self.child(blent_config::commands::OwnedChild::spawn(command)?))
    }
    /// Adapt an already-owned native child before exposing or awaiting it.
    pub fn child(
        &self,
        child: blent_config::commands::OwnedChild,
    ) -> blent_config::commands::OwnedChild {
        self.retain(child.completion());
        child
    }
    pub async fn output(
        &self,
        command: &mut tokio::process::Command,
        input: Option<&[u8]>,
        timeout: std::time::Duration,
    ) -> std::io::Result<std::process::Output> {
        use blent_config::commands::AsyncCommandExt;
        command
            .output_input_timeout_observed(input, timeout, &|proof| self.retain(proof))
            .await
    }
    pub fn sync_output(
        &self,
        command: &mut std::process::Command,
        timeout: std::time::Duration,
    ) -> std::io::Result<std::process::Output> {
        use blent_config::commands::SyncCommandExt;
        command.output_timeout_observed(timeout, &|proof| self.retain(proof))
    }
    /// Retain the original blocking creator until its actual join, even when
    /// canceled or when work releases its own reporting alias before cleanup.
    pub async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(Self) -> T + Send + 'static,
    ) -> anyhow::Result<T> {
        let creator = self.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let task = crate::owned_task::Task::spawn_blocking(move || {
            let value = work(creator);
            let _ = sender.send(value);
            Ok(())
        })?;
        self.retain(task.completion());
        task.await??;
        Ok(receiver.await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};
    struct Release(mpsc::Sender<()>);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }
    #[tokio::test]
    async fn t1032_cancelled_blocking_creator_survives_released_reporting_alias() {
        let (report, observation) = crate::capture_retirement::channel();
        let scope = Scope::new(Some(report));
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (release, held) = mpsc::channel();
        let release = Release(release);
        let (exited_tx, mut exited) = tokio::sync::oneshot::channel();
        let work = tokio::spawn(async move {
            scope
                .blocking(move |report| {
                    drop(report);
                    entered_tx.send(()).unwrap();
                    held.recv().unwrap();
                    exited_tx.send(()).unwrap();
                })
                .await
        });
        entered.await.unwrap();
        work.abort();
        assert!(work.await.unwrap_err().is_cancelled());
        let retained = !observation.complete();
        assert!(
            exited.try_recv().is_err(),
            "T1032 original creator must still be held"
        );
        drop(release);
        exited.await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !observation.complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            retained,
            "T1032 parent cancellation released original blocking creator"
        );
    }
    #[tokio::test]
    async fn t1032_blocking_results_and_panics_keep_their_original_meaning() {
        let scope = Scope::default();
        assert_eq!(scope.blocking(|_| 17).await.unwrap(), 17);
        assert!(scope
            .blocking(|_| panic!("T1032 worker panic"))
            .await
            .is_err());
    }
}

#[cfg(test)]
mod child_tests {
    use super::*;
    use std::{
        sync::{mpsc, Mutex},
        time::Duration,
    };
    struct Held(mpsc::Sender<()>, Mutex<mpsc::Receiver<()>>);
    impl Drop for Held {
        fn drop(&mut self) {
            let _ = self.0.send(());
            let _ = self.1.lock().unwrap().recv();
        }
    }
    struct Release(mpsc::Sender<()>);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }
    #[tokio::test]
    async fn t1032_original_probe_child_cleanup_outlives_scope_drop() {
        let (report, observed) = crate::capture_retirement::channel();
        let scope = Scope::new(Some(report));
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command.args(["--list"]).stdout(std::process::Stdio::null());
        let mut child = scope.spawn(&mut command).unwrap();
        let original = child.completion();
        let (entered_tx, entered) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let release = Release(release);
        child.keep_alive(Held(entered_tx, Mutex::new(held)));
        drop(child);
        drop(scope);
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        let retained = !observed.complete();
        assert!(!original.is_complete());
        drop(release);
        tokio::time::timeout(Duration::from_secs(5), async {
            while !observed.complete() || !original.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            retained,
            "T1032 native probe scope lost actual child cleanup"
        );
    }
    struct Forward {
        original: Arc<dyn NativeRetirement>,
        proofs: mpsc::Sender<Completion>,
    }
    impl NativeRetirement for Forward {
        fn retain(&self, proof: Completion) {
            self.original.retain(proof.clone());
            self.proofs.send(proof).unwrap();
        }
    }
    async fn completed(proof: &Completion) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !proof.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn t1032_cancelled_creator_reports_original_child_exposed_later() {
        let (original, observed) = crate::capture_retirement::channel();
        let (proof_tx, proofs) = mpsc::channel();
        let scope = Scope::new(Some(Arc::new(Forward {
            original,
            proofs: proof_tx,
        })));
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (create, blocked) = mpsc::channel();
        let create = Release(create);
        let (cleanup_tx, cleanup) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let release = Release(release);
        let (child_tx, child) = tokio::sync::oneshot::channel();
        let work = tokio::spawn(async move {
            scope
                .blocking(move |scope| {
                    entered_tx.send(()).unwrap();
                    blocked.recv().unwrap();
                    let mut command =
                        tokio::process::Command::new(std::env::current_exe().unwrap());
                    command.arg("--list").stdout(std::process::Stdio::null());
                    let mut child = scope.spawn(&mut command).unwrap();
                    child.keep_alive(Held(cleanup_tx, Mutex::new(held)));
                    child_tx.send(child.completion()).unwrap();
                    drop(child);
                })
                .await
        });
        entered.await.unwrap();
        let creator = proofs.recv_timeout(Duration::from_secs(5)).unwrap();
        work.abort();
        assert!(work.await.unwrap_err().is_cancelled());
        drop(create);
        let child = child.await.unwrap();
        completed(&creator).await;
        cleanup.recv_timeout(Duration::from_secs(5)).unwrap();
        let retained = !observed.complete();
        assert!(!child.is_complete());
        drop(release);
        completed(&child).await;
        tokio::time::timeout(Duration::from_secs(5), async {
            while !observed.complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert!(
            retained,
            "T1032 late native child escaped original canceled creator"
        );
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;
    use std::{sync::Mutex, time::Duration};
    #[derive(Default)]
    struct Report(Mutex<Vec<Completion>>);
    impl NativeRetirement for Report {
        fn retain(&self, proof: Completion) {
            self.0.lock().unwrap().push(proof);
        }
    }
    #[tokio::test]
    async fn t1032_scope_forwards_exact_sync_and_async_command_proofs() {
        let report = Arc::new(Report::default());
        let scope = Scope::new(Some(report.clone()));
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.arg("--list");
        assert!(scope
            .sync_output(&mut command, Duration::from_secs(5))
            .unwrap()
            .status
            .success());
        let mut command = tokio::process::Command::from(command);
        assert!(scope
            .output(&mut command, None, Duration::from_secs(5))
            .await
            .unwrap()
            .status
            .success());
        let originals = report.0.lock().unwrap().clone();
        tokio::time::timeout(Duration::from_secs(5), async {
            while originals.iter().any(|proof| !proof.is_complete()) {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            originals.len(),
            4,
            "T1032 capture command lost original child/collector proofs"
        );
    }
}
