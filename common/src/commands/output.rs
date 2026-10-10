//! One bounded collector owns both nonblocking pipes. Never collect on an executor.
use super::{
    output_limits::{Bytes, Limits},
    output_pipe::Reader,
    timed_out,
};
#[cfg(test)]
use crate::retirement::Completion;
use crate::retirement::{Admission, Pool};
use std::{
    io::{self, Read},
    process::{Command, ExitStatus, Output, Stdio},
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, OnceLock,
    },
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
const RUNNING: u8 = 0;
const FINISHED: u8 = 1;
const CANCELLED: u8 = 2;
type Sources = [Box<dyn Read + Send>; 2];
type Collected = io::Result<[Vec<u8>; 2]>;

fn reserve() -> io::Result<Admission> {
    static POOL: OnceLock<io::Result<Pool>> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(crate::retirement_limits::configured(
            "BLENT_COMMAND_COLLECTORS",
            128,
            4096,
        )?)
    })
    .as_ref()
    .map_err(|error| io::Error::new(error.kind(), error.to_string()))?
    .reserve()
}

pub(super) struct CapturedOutput {
    state: Arc<AtomicU8>,
    receiver: oneshot::Receiver<Collected>,
    #[cfg(test)]
    completion: Completion,
}
impl CapturedOutput {
    pub fn new(command: &mut Command, deadline: Instant) -> io::Result<Self> {
        let limits = Limits::environment()?;
        let admission = reserve()?;
        let (stdout, out) = io::pipe()?;
        let (stderr, err) = io::pipe()?;
        let sources: Sources = [
            Box::new(Reader::new(stdout)?),
            Box::new(Reader::new(stderr)?),
        ];
        command.stdout(Stdio::from(out)).stderr(Stdio::from(err));
        Self::start(sources, limits, deadline, admission)
    }
    fn start(
        sources: Sources,
        limits: Limits,
        deadline: Instant,
        admission: Admission,
    ) -> io::Result<Self> {
        let state = Arc::new(AtomicU8::new(RUNNING));
        let owner = state.clone();
        let (sender, receiver) = oneshot::channel();
        #[cfg(test)]
        let completion = admission.completion();
        let handle = std::thread::Builder::new()
            .name("blent-output".into())
            .spawn(move || {
                let result = collect(sources, limits, deadline, &owner);
                let _ = sender.send(result);
            })?;
        admission.retain(crate::retirement_work::Thread::new(handle));
        Ok(Self {
            state,
            receiver,
            #[cfg(test)]
            completion,
        })
    }
    pub fn child_exited(&self) {
        let _ = self
            .state
            .compare_exchange(RUNNING, FINISHED, Ordering::AcqRel, Ordering::Acquire);
    }
    pub fn poll_result(&mut self) -> Option<Collected> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(oneshot::error::TryRecvError::Empty) => None,
            Err(oneshot::error::TryRecvError::Closed) => Some(Err(collector_lost())),
        }
    }
    pub async fn result(&mut self) -> Collected {
        (&mut self.receiver).await.map_err(|_| collector_lost())?
    }
    pub fn finish_sync(&mut self, status: ExitStatus, deadline: Instant) -> io::Result<Output> {
        self.child_exited();
        loop {
            if let Some(result) = self.poll_result() {
                return result.map(|bytes| output(status, bytes));
            }
            if Instant::now() >= deadline {
                return Err(timed_out());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    pub async fn finish_async(&mut self, status: ExitStatus) -> io::Result<Output> {
        self.child_exited();
        Ok(output(status, self.result().await?))
    }
}
impl Drop for CapturedOutput {
    fn drop(&mut self) {
        self.state.store(CANCELLED, Ordering::Release);
    }
}
fn output(status: ExitStatus, [stdout, stderr]: [Vec<u8>; 2]) -> Output {
    Output {
        status,
        stdout,
        stderr,
    }
}
fn collector_lost() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "command output collector stopped",
    )
}

fn collect(mut sources: Sources, limits: Limits, deadline: Instant, state: &AtomicU8) -> Collected {
    let mut bytes = Bytes::new(limits);
    let mut closed = [false; 2];
    let mut scratch = [0u8; 16384];
    loop {
        let observed = state.load(Ordering::Acquire);
        if observed == CANCELLED {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if Instant::now() >= deadline {
            return Err(timed_out());
        }
        let mut progress = false;
        for (index, source) in sources.iter_mut().enumerate() {
            progress |= poll_source(
                source.as_mut(),
                &mut closed[index],
                &mut scratch,
                &mut bytes,
                index,
            )?;
        }
        if !progress {
            if observed == FINISHED {
                return Ok(bytes.streams);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
fn read_available(
    source: &mut dyn Read,
    closed: &mut bool,
    scratch: &mut [u8],
) -> io::Result<usize> {
    if *closed {
        return Ok(0);
    }
    match source.read(scratch) {
        Ok(0) => {
            *closed = true;
            Ok(0)
        }
        Ok(size) => Ok(size),
        Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock) => Ok(0),
        Err(error) => Err(error),
    }
}

fn poll_source(
    source: &mut dyn Read,
    closed: &mut bool,
    scratch: &mut [u8],
    bytes: &mut Bytes,
    index: usize,
) -> io::Result<bool> {
    match read_available(source, closed, scratch) {
        Ok(size) => {
            bytes.append(index, &scratch[..size])?;
            Ok(size > 0)
        }
        Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(true),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests;
