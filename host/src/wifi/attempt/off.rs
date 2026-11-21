//! An explicit off request owns no mutation until its first disconnect starts.
use super::*;
use blent_config::adb_reverse::temporary::Creation;

#[derive(Debug)]
pub(crate) struct Unavailable;
impl std::fmt::Display for Unavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ADB observation did not start")
    }
}
impl std::error::Error for Unavailable {}

async fn present<C: Commands>(adb: &Adb<C>, address: &str) -> Result<bool> {
    let command = adb.0.execute_route(vec!["devices".into()]).await;
    if !command.started {
        return Err(Unavailable.into());
    }
    Ok(cleanup::observed(command.output?, address)?.is_some())
}

pub(super) async fn run<C: Commands>(
    adb: &Adb<C>,
    address: &str,
    ready: oneshot::Sender<Result<()>>,
) {
    match present(adb, address).await {
        Ok(true) => {}
        Ok(false) => {
            let _ = ready.send(Ok(()));
            return;
        }
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    }
    let mut state = Creation::Acknowledged;
    let result = cleanup::remove(adb, address, &mut state).await;
    // Let the caller observe bounded settlement. A lost disconnect outcome
    // still retains this exact endpoint and original native cleanup sponsor.
    let _ = ready.send(result);
    if state == Creation::RemovalPending {
        cleanup::retire(adb, address, state).await;
    }
}
