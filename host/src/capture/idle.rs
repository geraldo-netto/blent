//! Linux capture adapter for portable T492/T1115 policies. Requests expire after 1.5s.
use super::CaptureConfig;
use crate::latency::{EncoderEvidence, LatencyTracker};
use anyhow::{Context, Result};
use blent_config::idle::control::{Cadence, LEASE_US};
use blent_config::idle::{unchanged::Conditions, Mode, Sample, COMPATIBLE_MS};
use policy::{Phase, Policy};
use std::{
    collections::VecDeque,
    io::Read,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::Instant;
mod directory;
mod policy;
mod staging;
mod subjects;
mod worker;

fn retire_request(mut cleanup: impl FnMut() -> Result<()>, mut pause: impl FnMut()) {
    let mut warned = false;
    while let Err(error) = cleanup() {
        if !warned {
            tracing::warn!(%error, "Idle request cleanup pending; original identities remain retained");
            warned = true;
        }
        pause();
    }
}

const MAX_TIMINGS: usize = 256;
static LEASE_FILES: Mutex<()> = Mutex::new(());
#[derive(Clone, Copy)]
struct Encoded {
    sequence: u32,
    pts_us: Option<i64>,
    ready: Instant,
    keyframe: bool,
}
type Timings = Arc<Mutex<VecDeque<Encoded>>>;

pub(super) struct Handle {
    timings: Timings,
    // Retain this view through Handle destruction, before task retirement.
    _lease: Arc<Lease>,
    // Lease views drop before native retirement exposes final dependency cleanup.
    task: worker::Worker,
}
impl Handle {
    pub(super) fn completion(&self) -> blent_config::retirement::Completion {
        self.task.completion()
    }
    pub(super) fn note(&self, sequence: u32, pts_us: Option<i64>, keyframe: bool) {
        let mut timings = self.timings.lock().unwrap();
        timings.push_back(Encoded {
            sequence,
            pts_us,
            ready: Instant::now(),
            keyframe,
        });
        if timings.len() > MAX_TIMINGS {
            timings.pop_front();
        }
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(super) async fn start(
    config: &CaptureConfig,
    evidence: Arc<EncoderEvidence>,
    tracker: LatencyTracker,
    viewers: Arc<dyn blent::capture_contract::ViewerActivity>,
    scope: &blent::native_scope::Scope,
) -> Option<Handle> {
    if !config.adaptive_idle {
        return None;
    }
    start_mode(Mode::Adaptive, config, evidence, tracker, viewers, scope).await
}

// Internal original-session choice. Configuration/UI selection remains T1111.
pub(super) async fn start_mode(
    mode: Mode,
    config: &CaptureConfig,
    evidence: Arc<EncoderEvidence>,
    tracker: LatencyTracker,
    viewers: Arc<dyn blent::capture_contract::ViewerActivity>,
    scope: &blent::native_scope::Scope,
) -> Option<Handle> {
    if mode == Mode::Compatible || evidence.decoder.is_none() {
        return None;
    }
    let subjects = subjects::Subjects::new(mode, viewers);
    if mode == Mode::Unchanged && !unchanged_available(config, &evidence, &subjects) {
        tracing::warn!("Unchanged-frame admission unavailable; retaining compatible cadence");
        return None;
    }
    let instance = config.instance;
    let stop = Arc::new(worker::Stop::default());
    let create_stop = stop.clone();
    let startup = spawn(
        mode,
        stop,
        move || Lease::open(&super::fifo_path_for(instance)?, create_stop),
        evidence,
        tracker,
        subjects,
        scope,
    );
    match finish_startup(startup).await {
        Ok(handle) => Some(handle),
        Err(error) => {
            tracing::warn!("Adaptive idle unavailable: {error}");
            None
        }
    }
}
fn unchanged_available(
    config: &CaptureConfig,
    evidence: &EncoderEvidence,
    subjects: &subjects::Subjects,
) -> bool {
    evidence.active() && policy::qualified(config, evidence) && subjects.ready()
}

// Queue views must drop before the worker retires its original held lease.
struct Startup {
    ready: tokio::sync::oneshot::Receiver<Arc<Lease>>,
    timings: Timings,
    task: Option<worker::Worker>,
}
impl Startup {
    async fn finish(mut self) -> Result<Handle> {
        let lease = (&mut self.ready)
            .await
            .context("native idle startup failed")?;
        Ok(Handle {
            timings: self.timings.clone(),
            _lease: lease,
            task: self.task.take().unwrap(),
        })
    }
}
async fn finish_startup(startup: Result<Startup>) -> Result<Handle> {
    startup?.finish().await
}
fn spawn(
    mode: Mode,
    stop: Arc<worker::Stop>,
    create: impl FnOnce() -> Result<Lease> + Send + 'static,
    evidence: Arc<EncoderEvidence>,
    tracker: LatencyTracker,
    subjects: subjects::Subjects,
    scope: &blent::native_scope::Scope,
) -> Result<Startup> {
    let timings = Timings::default();
    let work_timings = timings.clone();
    let runtime = tokio::runtime::Handle::current();
    let origin = Instant::now();
    let (ready, receiver) = tokio::sync::oneshot::channel();
    let running = stop.clone();
    let task = worker::Worker::spawn(stop, scope, move |holder| {
        if !running.active() {
            return Ok(());
        }
        let lease = Arc::new(create()?);
        holder.retain(lease.clone())?;
        if ready.send(lease.clone()).is_err() {
            return Ok(());
        }
        runtime.block_on(run(
            mode,
            lease,
            work_timings,
            evidence,
            tracker,
            origin,
            subjects,
        ));
        Ok(())
    })?;
    Ok(Startup {
        ready: receiver,
        timings,
        task: Some(task),
    })
}

async fn run(
    mode: Mode,
    lease: Arc<Lease>,
    timings: Timings,
    evidence: Arc<EncoderEvidence>,
    tracker: LatencyTracker,
    origin: Instant,
    subjects: subjects::Subjects,
) {
    let mut updates = tracker.activity_updates();
    let mut timer = tokio::time::interval(Duration::from_millis(250));
    let mut ordinal = 0;
    let recipe = mode != Mode::Unchanged || policy::recipe(&evidence);
    let mut policy = Policy::new(mode, recipe, evidence.decoder.is_some());
    let mut previous = policy.phase();
    while evidence.active() && lease.stop.active() {
        let viewer_change = subjects.changed(mode);
        if !subjects.same_epoch() {
            lease.clear();
            return;
        }
        let conditions = subjects.current(mode, &evidence, &lease.stop);
        consume(
            &mut policy,
            conditions,
            &timings,
            &evidence,
            &mut ordinal,
            origin,
        );
        policy.tick(
            subjects.current(mode, &evidence, &lease.stop),
            micros(origin, Instant::now()),
        );
        if policy.phase() != previous {
            tracing::info!(
                "Idle policy {mode:?}: {:?}, cadence={}ms, encoder={}",
                policy.phase(),
                policy.interval_ms(),
                evidence.name
            );
            previous = policy.phase();
        }
        if let Err(error) = publish_policy(&lease, &mut policy, mode, &subjects, &evidence, origin)
        {
            tracing::warn!("Idle policy restored compatibility: {error}");
            lease.clear();
            return;
        }
        if policy.phase() == Phase::Rejected {
            return;
        }
        tokio::select! { _ = timer.tick() => {}, _ = updates.changed() => {}, _ = lease.stop.changed() => {}, _ = viewer_change => {} }
    }
    lease.clear();
}

fn publish_policy(
    lease: &Lease,
    policy: &mut Policy,
    mode: Mode,
    subjects: &subjects::Subjects,
    evidence: &EncoderEvidence,
    origin: Instant,
) -> Result<()> {
    lease.publish_while(policy.interval_ms(), || {
        policy.tick(
            subjects.current(mode, evidence, &lease.stop),
            micros(origin, Instant::now()),
        );
        policy.phase() != Phase::Rejected && subjects.permits(mode, evidence, &lease.stop)
    })
}

fn consume(
    policy: &mut Policy,
    conditions: Conditions,
    timings: &Timings,
    evidence: &EncoderEvidence,
    ordinal: &mut u64,
    origin: Instant,
) {
    for ack in evidence.samples_after(*ordinal) {
        if ack.ordinal != *ordinal + 1 {
            policy.reject();
            return;
        }
        *ordinal = ack.ordinal;
        let encoded = take(timings, ack.sequence);
        match encoded.and_then(|frame| {
            frame.pts_us.map(|pts_us| Sample {
                sequence: frame.sequence,
                pts_us,
                ready_us: micros(origin, frame.ready),
                ack_us: micros(origin, ack.at),
                keyframe: frame.keyframe,
            })
        }) {
            Some(sample) => policy.observe(conditions, sample),
            None => {
                policy.reject();
                return;
            }
        }
    }
}

fn take(timings: &Timings, sequence: u32) -> Option<Encoded> {
    let mut timings = timings.lock().unwrap();
    let position = timings
        .iter()
        .position(|frame| frame.sequence == sequence)?;
    timings.drain(..position);
    timings.pop_front()
}

fn micros(origin: Instant, at: Instant) -> u64 {
    at.saturating_duration_since(origin)
        .as_micros()
        .min(u64::MAX as u128) as u64
}

struct Lease {
    path: PathBuf,
    identity: (u64, u64),
    token: u64,
    stop: Arc<worker::Stop>,
    _inode: std::fs::File,
    _token: std::fs::File,
    directory: directory::Directory,
    #[cfg(test)]
    publication_observer: Mutex<Option<Arc<publication_tests::Hook>>>,
}
impl Lease {
    fn open(fifo: &Path, stop: Arc<worker::Stop>) -> Result<Self> {
        let directory = directory::Directory::open(fifo.parent().context("FIFO directory")?)?;
        let held = directory.lock()?;
        staging::collect(&held)?;
        let inode = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(directory.path(fifo)?)?;
        let metadata = inode.metadata()?;
        anyhow::ensure!(
            metadata.file_type().is_fifo(),
            "idle request needs an owned FIFO"
        );
        let token = tempfile::tempfile_in(directory.root())?;
        drop(held);
        Ok(Self {
            path: fifo.with_extension("idle"),
            identity: (metadata.dev(), metadata.ino()),
            token: token.metadata()?.ino(),
            stop,
            _inode: inode,
            _token: token,
            directory,
            #[cfg(test)]
            publication_observer: Mutex::new(None),
        })
    }
}

impl Cadence for Lease {
    fn publish(&self, interval: u32) -> Result<()> {
        self.publish_while(interval, || true)
    }

    fn clear(&self) {
        let _ = self.try_clear();
    }
}

impl Lease {
    fn try_clear(&self) -> Result<()> {
        let _guard = LEASE_FILES.lock().unwrap();
        self.clear_locked().with_context(|| {
            format!(
                "idle request {:?}, source {:?}, token {}",
                self.path, self.identity, self.token
            )
        })
    }
}

impl Lease {
    // Predicate is a synchronous original observation, never a native fence.
    // Recheck after native waits and staging IO, before the advisory rename.
    fn publish_while(&self, interval: u32, mut current: impl FnMut() -> bool) -> Result<()> {
        if interval == COMPATIBLE_MS {
            self.clear();
            return Ok(());
        }
        anyhow::ensure!(self.stop.active(), "idle lease retired");
        let _guard = LEASE_FILES.lock().unwrap();
        #[cfg(test)]
        let _observation =
            publication_tests::Publication::new(self.publication_observer.lock().unwrap().take());
        self.publish_locked(interval, || {}, &mut current)
    }

    fn check_current(&self, current: &mut impl FnMut() -> bool) -> Result<()> {
        if !self.stop.active() {
            anyhow::bail!("idle lease retired");
        }
        if !current() {
            anyhow::bail!("original idle admission changed");
        }
        Ok(())
    }

    fn publish_locked(
        &self,
        interval: u32,
        before_wait: impl FnOnce(),
        current: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        anyhow::ensure!(self.stop.active(), "idle lease retired");
        anyhow::ensure!(matches!(interval, 500 | 1000), "unsupported sparse cadence");
        before_wait();
        let directory = self.directory.lock()?;
        self.check_current(current)?;
        self.write_request(interval, &directory, current)
    }

    fn write_request(
        &self,
        interval: u32,
        directory: &directory::Guard<'_>,
        current: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        let mut clock = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // Linux CLOCK_MONOTONIC is also used by the helper; no wall-clock lease.
        anyhow::ensure!(
            unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut clock) } == 0,
            "monotonic clock unavailable"
        );
        let expires =
            clock.tv_sec as u64 * 1000 + clock.tv_nsec as u64 / 1_000_000 + LEASE_US / 1000;
        let mut file = staging::Staging::create(directory)?;
        writeln!(
            file.writer(),
            "{} {} {} {expires} {interval}",
            self.identity.0,
            self.identity.1,
            self.token
        )?;
        self.check_current(current)?;
        match file.persist(&self.path) {
            Ok(persisted) => {
                drop(persisted);
                Ok(())
            }
            Err(cause) => Err(cause).context("publish idle request"),
        }
    }
}

impl Lease {
    fn contents(&self) -> std::io::Result<String> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(self.directory.path(&self.path)?)?;
        let mut text = String::new();
        file.take(160).read_to_string(&mut text)?;
        Ok(text)
    }

    fn clear_locked(&self) -> Result<()> {
        self.clear_locked_with(|| {})
    }

    fn clear_locked_with(&self, after_snapshot: impl FnOnce()) -> Result<()> {
        self.clear_using(after_snapshot, |path| std::fs::remove_file(path))
    }

    fn clear_using(
        &self,
        after_snapshot: impl FnOnce(),
        remove: impl FnOnce(&Path) -> std::io::Result<()>,
    ) -> Result<()> {
        let _directory = self
            .directory
            .lock()
            .context("lock original idle directory")?;
        if !self.original_request()? {
            return Ok(());
        }
        after_snapshot();
        let path = self.directory.path(&self.path)?;
        directory::remove_owned(&path, remove).context("remove original idle request")
    }

    fn original_request(&self) -> Result<bool> {
        let info = match std::fs::symlink_metadata(self.directory.path(&self.path)?) {
            Ok(info) => info,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if !info.file_type().is_file() {
            return Ok(false); // C cannot read advice through this replacement.
        }
        self.owns_cookie(&self.contents()?)
            .context("unknown original idle request")
    }

    fn owns_cookie(&self, text: &str) -> Option<bool> {
        let mut fields = text.split_ascii_whitespace();
        let identity = (
            fields.next()?.parse::<u64>().ok()?,
            fields.next()?.parse::<u64>().ok()?,
        );
        let token = fields.next()?.parse::<u64>().ok()?;
        Some(identity == self.identity && token == self.token)
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        retire_request(
            || self.try_clear(),
            || std::thread::sleep(Duration::from_millis(100)),
        );
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lease_tests;

#[cfg(test)]
mod retirement_tests;

#[cfg(test)]
mod trial_tests;

#[cfg(test)]
mod interop_tests;

#[cfg(test)]
mod publication_tests;
