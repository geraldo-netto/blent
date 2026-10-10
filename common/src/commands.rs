//! Command deadlines, not transactional cancellation of delegated operations.
//! Linux commands get a private process group. Timeout/cancellation signals its
//! members and reaps the direct child when permitted. Windows commands enter
//! an owned kill-on-close job before their suspended initial thread resumes.
//! Work delegated to another service can outlive the owned process tree.
use std::io;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::time::Duration;

mod output;
mod output_limits;
mod output_pipe;
mod owned;
use output::CapturedOutput;
mod retirement;
#[cfg(windows)]
mod windows;
pub use owned::{OwnedChild, Retirement, RetirementError, RetirementPhase};

pub fn spawn_reaped(command: &mut std::process::Command) -> std::io::Result<u32> {
    #[cfg(windows)]
    windows::child_priority(command, 0);
    let admission = retirement::reserve()?;
    let child = command.spawn()?;
    let pid = child.id();
    admission.retain(retirement::SyncChild(child));
    Ok(pid)
}

pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

/// Grace allowed by the daemon's `stop` command.
pub const DAEMON_STOP_TIMEOUT: Duration = Duration::from_secs(10);
/// Matches TimeoutStopSec in both shipped systemd user units (T268).
pub const SERVICE_STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// A service stop may spend one budget on ExecStop and another retiring the
/// service process. Leave a further command budget for dispatch/restart.
pub fn daemon_command_timeout(managed: bool) -> Duration {
    if managed {
        SERVICE_STOP_TIMEOUT * 2 + COMMAND_TIMEOUT
    } else {
        DAEMON_STOP_TIMEOUT + COMMAND_TIMEOUT
    }
}

pub trait SyncCommandExt {
    fn output_bounded(&mut self) -> io::Result<Output> {
        self.output_timeout(COMMAND_TIMEOUT)
    }
    fn output_timeout(&mut self, timeout: Duration) -> io::Result<Output>;
}
fn command_deadline(timeout: Duration) -> io::Result<std::time::Instant> {
    std::time::Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "command timeout is out of range",
            )
        })
}

fn timed_out() -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        "command timed out; detached or privileged work may still be running",
    )
}

fn prepare_group(command: &mut Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = command;
    #[cfg(windows)]
    {
        windows::child_priority(
            command,
            windows_sys::Win32::System::Threading::CREATE_SUSPENDED,
        );
    }
}

fn terminate_group(pid: u32) {
    #[cfg(target_os = "linux")]
    {
        // The direct child is still owned and unreaped: its PID cannot have
        // been reused. prepare_group made that PID the new group's identity.
        let result = unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
        if result < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            tracing::warn!(
                pid,
                "Command group could not be terminated; delegated work may continue"
            );
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = pid;
}

fn retire_sync(mut child: std::process::Child, admission: crate::retirement::Admission) {
    if matches!(child.try_wait(), Ok(Some(_))) {
        return;
    }
    terminate_group(child.id());
    let signalled = child.kill();
    reap_reserved(child, signalled, admission);
}
#[cfg(test)]
fn reap_after_signal(child: std::process::Child, signalled: io::Result<()>) {
    reap_reserved(child, signalled, retirement::reserve().unwrap());
}
fn reap_reserved(
    mut child: std::process::Child,
    signalled: io::Result<()>,
    admission: crate::retirement::Admission,
) {
    // Poll only through the configured control budget, even after a successful
    // signal. A failed signal never authorizes an unbounded wait either.
    let budget = crate::retirement_limits::reap_budget().unwrap_or(Duration::from_millis(100));
    let end = std::time::Instant::now() + budget;
    while signalled.is_ok() && std::time::Instant::now() < end {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    admission.retain(retirement::SyncChild(child));
}

impl SyncCommandExt for Command {
    fn output_timeout(&mut self, timeout: Duration) -> io::Result<Output> {
        let admission = retirement::reserve()?;
        crate::retirement_limits::reap_budget()?;
        let deadline = command_deadline(timeout)?;
        let mut output = CapturedOutput::new(self, deadline)?;
        #[cfg(windows)]
        let job = windows::Job::new()?;
        prepare_group(self);
        let mut child = self.stdin(Stdio::null()).spawn()?;
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            if let Err(error) = job.start(child.id(), child.as_raw_handle()) {
                retire_sync(child, admission);
                return Err(error);
            }
        }
        let result = wait_collected(&mut child, &mut output, deadline);
        if result.is_err() {
            retire_sync(child, admission);
        }
        result
    }
}
fn wait_collected(
    child: &mut std::process::Child,
    output: &mut CapturedOutput,
    deadline: std::time::Instant,
) -> io::Result<Output> {
    loop {
        if let Some(result) = output.poll_result() {
            return Err(result
                .err()
                .unwrap_or_else(|| io::Error::other("collector completed before child")));
        }
        if let Some(status) = child.try_wait()? {
            return output.finish_sync(status, deadline);
        }
        if std::time::Instant::now() >= deadline {
            return Err(timed_out());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub trait AsyncCommandExt {
    fn output_bounded(&mut self) -> impl std::future::Future<Output = io::Result<Output>> + Send {
        self.output_timeout(COMMAND_TIMEOUT)
    }
    fn output_timeout(
        &mut self,
        timeout: Duration,
    ) -> impl std::future::Future<Output = io::Result<Output>> + Send;
    /// Feed sensitive command data through stdin, never process arguments.
    fn output_input_timeout(
        &mut self,
        input: Option<&[u8]>,
        timeout: Duration,
    ) -> impl std::future::Future<Output = io::Result<Output>> + Send;
}
impl AsyncCommandExt for tokio::process::Command {
    async fn output_timeout(&mut self, timeout: Duration) -> io::Result<Output> {
        self.output_input_timeout(None, timeout).await
    }

    async fn output_input_timeout(
        &mut self,
        input: Option<&[u8]>,
        timeout: Duration,
    ) -> io::Result<Output> {
        let deadline = command_deadline(timeout)?;
        let mut output = CapturedOutput::new(self.as_std_mut(), deadline)?;
        self.stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let mut child = OwnedChild::spawn(self)?;
        let operation = async {
            let status = tokio::select! {
                status = feed_and_wait(&mut child,input) => status?,
                result = output.result() => return Err(result.err().unwrap_or_else(|| io::Error::other("collector completed before child"))),
            };
            output.finish_async(status).await
        };
        let result = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), operation)
            .await
            .unwrap_or_else(|_| Err(timed_out()));
        if result.is_err() {
            let retirement = child.retire(Duration::ZERO).await;
            if !retirement.completion.is_complete() {
                tracing::warn!("Command retirement incomplete; capacity remains reserved");
            }
        }
        result
    }
}

async fn feed_and_wait(child: &mut OwnedChild, input: Option<&[u8]>) -> io::Result<ExitStatus> {
    use tokio::io::AsyncWriteExt;
    if let Some(input) = input {
        let mut stdin = child.take_stdin().expect("piped command input");
        stdin.write_all(input).await?;
        stdin.shutdown().await?;
    }
    child.wait().await
}

#[cfg(test)]
mod tests;
