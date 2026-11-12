//! Route mutations retain original native command fences before policy observes
//! effects. A completed child is not proof of a delegated ADB-server outcome.
use super::*;
use blent_config::retirement::Completion;
use std::sync::Mutex;

pub struct RouteCommand {
    pub output: Result<Output>,
    /// False proves that no native creator was started, not merely no reply.
    pub started: bool,
}
pub type RouteFuture<'a> = Pin<Box<dyn Future<Output = RouteCommand> + Send + 'a>>;

pub(super) async fn native(program: &std::path::Path, arguments: Vec<String>) -> RouteCommand {
    let proofs = Mutex::new(Vec::<Completion>::new());
    let observer = |proof| {
        let mut owned = proofs.lock().unwrap();
        assert!(owned.len() < 2, "one route command child and collector");
        owned.push(proof);
    };
    let output = tokio::process::Command::new(program)
        .args(arguments)
        .output_input_timeout_observed(None, Duration::from_secs(5), &observer)
        .await
        .map_err(Into::into);
    let started = proofs.lock().unwrap().len() == 2;
    // The common adapter only certifies pre-spawn errors on Linux. Elsewhere,
    // a failed platform job attachment may already own an unreported child.
    #[cfg(not(target_os = "linux"))]
    if !started {
        std::future::pending::<()>().await;
    }
    while !proofs.lock().unwrap().iter().all(Completion::is_complete) {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    RouteCommand { output, started }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn t1363_native_route_commands_preserve_spawn_and_exit_evidence() {
        let root = tempfile::tempdir().unwrap();
        let failed = native(&root.path().join("missing-adb"), vec![]).await;
        assert!(!failed.started);
        assert!(failed.output.is_err());
        for code in [0, 7] {
            let result = native(
                std::path::Path::new("/bin/sh"),
                vec!["-c".into(), format!("exit {code}")],
            )
            .await;
            assert!(result.started);
            assert_eq!(result.output.unwrap().status.code(), Some(code));
        }
        let some = Some(NativeCommands("/bin/true".into()));
        assert!(some
            .execute_route(vec![])
            .await
            .output
            .unwrap()
            .status
            .success());
        let none = None::<NativeCommands>;
        let failed = none.execute_route(vec![]).await;
        assert!(!failed.started);
        assert!(failed.output.is_err());
    }
}
