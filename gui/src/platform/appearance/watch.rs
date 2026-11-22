//! One native subscription process, one admitted worker and one replaceable
//! snapshot. Drop cancels reads; the original child and worker retain their
//! independent retirement admissions until native reap/join completes.
use super::{font, parse, Appearance};
use std::{
    io,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, sync::oneshot};
type Snapshot = (Appearance, Vec<u8>);
type Update = Result<Snapshot, String>;
#[derive(Default)]
struct Latest {
    value: Mutex<Option<Update>>,
    #[cfg(test)]
    native: Mutex<Option<blent_config::retirement::Completion>>,
}
impl Latest {
    fn publish(&self, update: Update, notify: &impl Fn()) {
        let wake = self.value.lock().unwrap().replace(update).is_none();
        if wake {
            notify();
        }
    }
    fn take(&self) -> Option<Update> {
        self.value.lock().unwrap().take()
    }
}
pub(crate) struct Watch {
    latest: Arc<Latest>,
    stop: Option<oneshot::Sender<()>>,
    _worker: blent::control_workers::Worker,
}
impl Watch {
    pub(crate) fn start(notify: impl Fn() + Send + 'static) -> io::Result<Self> {
        let mut command = Command::new("/usr/bin/python3");
        command.args(["-I", "-c", include_str!("gtk.py"), "--watch"]);
        Self::start_command(command, notify)
    }
    fn start_command(mut command: Command, notify: impl Fn() + Send + 'static) -> io::Result<Self> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let latest = Arc::new(Latest::default());
        let output = latest.clone();
        let (stop, cancelled) = oneshot::channel();
        let pool = blent::control_workers::WorkerPool::new(1)?;
        let worker = blent::control_workers::Worker::spawn(pool, "blent-appearance", move || {
            let result = runtime.block_on(run(command, cancelled, &output, &notify));
            if let Err(error) = result {
                output.publish(Err(error.to_string()), &notify);
            }
        })?;
        Ok(Self {
            latest,
            stop: Some(stop),
            _worker: worker,
        })
    }
    pub(crate) fn take(&self) -> Option<Update> {
        self.latest.take()
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        // Worker::drop transfers its original join to the process retirement owner.
    }
}
async fn run(
    mut command: Command,
    mut cancelled: oneshot::Receiver<()>,
    output: &Latest,
    notify: &impl Fn(),
) -> io::Result<()> {
    let mut child = blent_config::commands::OwnedChild::spawn(&mut command)?;
    #[cfg(test)]
    {
        *output.native.lock().unwrap() = Some(child.completion());
    }
    let input = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("GTK appearance pipe unavailable"))?;
    let result = tokio::select! {
        biased;
        _ = &mut cancelled => Ok(()),
        result = receive(input, output, notify) => result,
    };
    // Closing owned stdin lets GTK unregister normally. Escalation and retained
    // exact-child observation are supplied by the existing command owner.
    child.retire(Duration::from_millis(250)).await;
    result
}
async fn receive(
    mut input: impl tokio::io::AsyncRead + Unpin,
    output: &Latest,
    notify: &impl Fn(),
) -> io::Result<()> {
    let mut previous = None;
    loop {
        let bytes = frame(&mut input).await?;
        match changed(&bytes, previous.as_ref()) {
            Ok(Some(snapshot)) => {
                previous = Some(snapshot.0.clone());
                output.publish(Ok(snapshot), notify);
            }
            Ok(None) => (),
            Err(reason) => {
                previous = None;
                output.publish(Err(reason), notify);
            }
        }
    }
}
fn changed(bytes: &[u8], previous: Option<&Appearance>) -> Result<Option<Snapshot>, String> {
    let appearance = parse(bytes, true)?;
    if previous == Some(&appearance) {
        return Ok(None);
    }
    let bytes = font(&appearance.font_family)?;
    Ok(Some((appearance, bytes)))
}

async fn frame(input: &mut (impl tokio::io::AsyncRead + Unpin)) -> io::Result<Vec<u8>> {
    let size = input.read_u32().await? as usize;
    if !(1..=8192).contains(&size) {
        return Err(io::Error::other("Invalid GTK appearance frame size"));
    }
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes).await?;
    Ok(bytes)
}
#[cfg(test)]
mod tests;
