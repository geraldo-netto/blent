//! Linux process adapter for the optional native encoder. Raw slots cross exec
//! by descriptor; only compressed access units cross the bounded stdout pipe.
use anyhow::{ensure, Context, Result};
use blent::encoder_worker_contract::{self as wire, Job};
use std::{
    ffi::OsString,
    io::{IoSlice, Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) struct Output {
    pub retirement: Option<Arc<dyn blent::capture_contract::NativeRetirement>>,
    pub video: blent::capture_contract::VideoOutput,
    pub headers: crate::media::CodecConfig,
    pub latency: crate::latency::LatencyTracker,
    pub refresh: Arc<AtomicBool>,
}
pub(crate) fn nonce() -> Result<u64> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| anyhow::anyhow!("Encoder worker nonce exhausted"))
}
pub(crate) fn spawn(
    job: Job,
    fifo: &Path,
    socket: Option<crate::raw_socket::Socket>,
    output: Output,
    guard: impl Send + Sync + 'static,
) -> Result<(blent_config::commands::OwnedChild, blent::owned_task::Task)> {
    let executable = std::env::current_exe()?;
    // Unit fixtures execute the separately built production entry point.
    #[cfg(test)]
    let executable = executable
        .parent()
        .and_then(Path::parent)
        .context("Missing test artifact directory")?
        .join("blent");
    spawn_at(&executable, job, fifo, socket, output, guard)
}
fn spawn_at(
    executable: &Path,
    job: Job,
    fifo: &Path,
    socket: Option<crate::raw_socket::Socket>,
    output: Output,
    guard: impl Send + Sync + 'static,
) -> Result<(blent_config::commands::OwnedChild, blent::owned_task::Task)> {
    let mut command = tokio::process::Command::new(executable);
    command
        .args([wire::ARGUMENT, &job.encode()?])
        .arg(fifo)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    if let Some(socket) = socket {
        socket.attach(&mut command)?;
    }
    let mut child = blent_config::commands::OwnedChild::spawn(&mut command)?;
    retain(&output.retirement, child.completion());
    child.keep_alive(guard);
    let input = child
        .take_stdin()
        .context("Encoder worker stdin unavailable")?;
    let stdout = child
        .take_stdout()
        .context("Encoder worker stdout unavailable")?;
    let retirement = output.retirement.clone();
    let handle = blent::owned_task::Task::spawn(receive(job, input, stdout, output))?;
    retain(&retirement, handle.completion());
    Ok((child, handle))
}
fn retain(
    retirement: &Option<Arc<dyn blent::capture_contract::NativeRetirement>>,
    original: blent_config::retirement::Completion,
) {
    if let Some(retirement) = retirement {
        retirement.retain(original);
    }
}

async fn receive(
    job: Job,
    mut input: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    output: Output,
) -> Result<()> {
    let refresh = output.refresh.clone();
    let controls = async move {
        loop {
            if refresh.swap(false, Ordering::AcqRel) {
                tokio::time::timeout(std::time::Duration::from_secs(1), input.write_all(b"I"))
                    .await??;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    tokio::select! { result = packets(&job, stdout, output) => result, result = controls => result }
}
async fn packets(
    job: &Job,
    mut input: impl tokio::io::AsyncRead + Unpin,
    output: Output,
) -> Result<()> {
    let generation = crate::media::EncoderGeneration::new();
    let evidence = output.latency.encoder_started_with_budget(
        &job.encoder,
        (job.width, job.height, job.fps, job.bitrate, job.quality),
        None,
        job.workers,
        None,
    );
    loop {
        let mut header = [0; 5];
        input
            .read_exact(&mut header)
            .await
            .context("Encoder worker ended before packet")?;
        let (size, is_idr) = wire::packet_size(header)?;
        let mut data = vec![0; size];
        input
            .read_exact(&mut data)
            .await
            .context("Truncated encoder worker packet")?;
        publish(&job.encoder, &output, &generation, &evidence, data, is_idr);
    }
}
fn publish(
    name: &str,
    output: &Output,
    generation: &crate::media::EncoderGeneration,
    evidence: &Arc<crate::latency::EncoderEvidence>,
    data: Vec<u8>,
    is_idr: bool,
) {
    if is_idr {
        if let Some(headers) = crate::encoder_io::extract_parameter_sets(
            &data,
            crate::media::Codec::from_encoder(name),
        ) {
            output.headers.publish(Some(headers));
        }
    }
    let seq = output.latency.next_sequence();
    if output.video.receiver_count() > 0 {
        output.latency.on_encoded_for(seq, evidence);
        let _ = output.video.send(crate::media::VideoPacket {
            data: data.into(),
            is_idr,
            seq,
            codec_config: output.headers.current(),
            generation: generation.active.clone(),
        });
    }
    output.latency.maybe_report();
}
pub(crate) fn dispatch(mut args: impl Iterator<Item = OsString>) -> Option<Result<()>> {
    if args.next()?.as_os_str() != wire::ARGUMENT {
        return None;
    }
    Some(arguments(args).and_then(|(job, path, socket)| run(job, &path, socket)))
}
fn arguments(
    mut args: impl Iterator<Item = OsString>,
) -> Result<(Job, std::path::PathBuf, Option<crate::raw_socket::Socket>)> {
    use std::os::unix::ffi::OsStrExt;
    let encoded = args.next().context("Missing encoder job")?;
    let job = Job::decode(encoded.as_bytes())?;
    let path = args.next().context("Missing encoder FIFO")?.into();
    let socket = match args.next() {
        Some(flag) => {
            ensure!(
                flag == "--capture-socket-fd",
                "Unexpected encoder worker argument"
            );
            let fd = args
                .next()
                .context("Missing raw descriptor")?
                .into_string()
                .map_err(|_| anyhow::anyhow!("Invalid raw descriptor"))?
                .parse()?;
            Some(crate::raw_socket::Socket::from_inherited(fd)?)
        }
        None => None,
    };
    ensure!(args.next().is_none(), "Extra encoder worker arguments");
    Ok((job, path, socket))
}
fn parent_guard(expected: u32) -> Result<()> {
    ensure!(
        unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } == 0,
        "Cannot bind encoder worker lifetime"
    );
    ensure!(
        unsafe { libc::getppid() } as u32 == expected,
        "Encoder parent already retired"
    );
    Ok(())
}
fn run(job: Job, fifo: &Path, socket: Option<crate::raw_socket::Socket>) -> Result<()> {
    parent_guard(job.parent)?;
    let stop = crate::encoder_io::StopSignal::new()?;
    let refresh = Arc::new(AtomicBool::new(true));
    let controlled_stop = stop.clone();
    let controlled_refresh = refresh.clone();
    std::thread::Builder::new()
        .name("blent-encoder-control".into())
        .spawn(move || control(std::io::stdin().lock(), controlled_refresh, controlled_stop))?;
    use std::os::fd::AsFd;
    let bytes = std::fs::File::from(std::io::stdout().as_fd().try_clone_to_owned()?);
    let output = blent::capture_contract::VideoOutput::new(Pipe {
        bytes: Mutex::new(bytes),
        stop: stop.clone(),
    });
    crate::encoder::run_with_nonce(
        fifo,
        &job.encoder,
        job.width,
        job.height,
        job.fps,
        job.bitrate,
        job.quality,
        output,
        Default::default(),
        refresh,
        stop,
        Default::default(),
        socket,
        job.slots,
        job.workers,
        &job.device,
        Some(job.nonce),
    )
}
fn control(
    mut input: impl Read,
    refresh: Arc<AtomicBool>,
    stop: Arc<crate::encoder_io::StopSignal>,
) {
    let mut byte = [0];
    while input.read_exact(&mut byte).is_ok() {
        if byte != *b"I" {
            break;
        }
        refresh.store(true, Ordering::Release);
    }
    stop.request();
}
struct Pipe {
    bytes: Mutex<std::fs::File>,
    stop: Arc<crate::encoder_io::StopSignal>,
}
struct Activity;
impl blent::capture_contract::ViewerActivity for Activity {
    fn epoch(&self) -> u64 {
        0
    }
}
impl blent::capture_contract::PacketOutput for Pipe {
    fn send(
        &self,
        packet: crate::media::VideoPacket,
    ) -> std::result::Result<usize, crate::media::VideoPacket> {
        let mut bytes = self.bytes.lock().unwrap();
        if write_packet(&mut *bytes, &packet.data, packet.is_idr).is_err() {
            self.stop.request();
            return Err(packet);
        }
        Ok(1)
    }
    fn receiver_count(&self) -> usize {
        1
    }
    fn viewers(&self) -> Arc<dyn blent::capture_contract::ViewerActivity> {
        Arc::new(Activity)
    }
}
fn write_packet(output: &mut impl Write, data: &[u8], key: bool) -> Result<()> {
    let header = wire::packet_header(data.len(), key)?;
    let mut parts = [IoSlice::new(&header), IoSlice::new(data)];
    let mut pending = &mut parts[..];
    while !pending.is_empty() {
        let count = match output.write_vectored(pending) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        ensure!(count != 0, "Encoder packet write made no progress");
        IoSlice::advance_slices(&mut pending, count);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
