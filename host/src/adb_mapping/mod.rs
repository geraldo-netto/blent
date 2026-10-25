//! Retained temporary mappings. External tools must leave admitted endpoints
//! unchanged through cleanup; unrelated ADB operations remain independent.
use anyhow::{ensure, Result};
use blent_config::retirement::{Admission, Completion, Observe, Pool};
use std::{
    io,
    net::TcpListener,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

mod native;
mod worker;

pub struct Target {
    pub adb: PathBuf,
    pub serial: String,
    pub remote: String,
    pub local: String,
}

impl Target {
    pub fn port(&self) -> Result<u16> {
        blent_config::adb_reverse::temporary::port(&self.remote)
    }

    #[cfg(test)]
    pub async fn test_shell(&self, command: String, failure: &str) -> Result<()> {
        use blent_config::commands::AsyncCommandExt;
        let result = tokio::process::Command::new(&self.adb)
            .args(["-s", &self.serial, "shell"])
            .output_input_timeout(Some(command.as_bytes()), Duration::from_secs(5))
            .await?;
        ensure!(result.status.success(), "{failure}");
        ensure!(
            crate::adb_command::broadcast_accepted(&result.stdout),
            "tablet did not accept media invitation"
        );
        Ok(())
    }
}

struct Control {
    target: Target,
    endpoints: Mutex<Option<Endpoints>>,
    commands: Mutex<Vec<Completion>>,
    settled: AtomicBool,
}

struct Endpoints {
    _remote: TcpListener,
    _local: TcpListener,
    #[cfg(test)]
    funding: Completion,
}

// Field destruction order keeps resources funded on rejected or unpolled starts.
struct Opening {
    endpoints: Endpoints,
    admission: Admission,
}

#[cfg(test)]
impl Drop for Endpoints {
    fn drop(&mut self) {
        assert!(
            !self.funding.is_complete(),
            "T1090 native listeners outlived their admission"
        );
    }
}

struct Invocation {
    input: String,
    result: oneshot::Sender<Result<(), String>>,
}

pub struct Lease {
    control: Arc<Control>,
    requests: mpsc::Sender<Invocation>,
    stop: watch::Sender<bool>,
    failure: watch::Receiver<Option<String>>,
    completion: Completion,
}

fn reserve() -> io::Result<Admission> {
    static POOL: OnceLock<io::Result<Pool>> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(blent_config::retirement_limits::configured(
            "BLENT_ADB_MAPPING_LIMIT",
            16,
            4096,
        )?)
    })
    .as_ref()
    .map_err(|error| io::Error::new(error.kind(), error.to_string()))?
    .reserve()
}

impl Lease {
    pub async fn create(
        adb: PathBuf,
        serial: Option<&str>,
        listener: &tokio::net::TcpListener,
        observe: &(dyn Fn(Completion) -> Result<()> + Send + Sync),
    ) -> Result<Self> {
        let local = listener.local_addr()?.port();
        ensure!(local != 0, "ADB media port must be nonzero");
        let serial = match serial {
            Some(serial) => serial.to_owned(),
            None => crate::adb_command::output(&adb, &["get-serialno"]).await?,
        };
        ensure!(!serial.trim().is_empty(), "missing tablet serial");
        let admission = reserve()?;
        observe(admission.completion())?;
        let local_listener = TcpListener::from(socket2::SockRef::from(listener).try_clone()?);
        let endpoint = TcpListener::bind("127.0.0.1:0")?;
        let remote = format!("tcp:{}", endpoint.local_addr()?.port());
        let target = Target {
            adb,
            serial,
            remote,
            local: format!("tcp:{local}"),
        };
        let endpoints = Endpoints {
            _remote: endpoint,
            _local: local_listener,
            #[cfg(test)]
            funding: admission.completion(),
        };
        Self::start(
            target,
            Opening {
                endpoints,
                admission,
            },
        )
        .await
    }

    async fn start(target: Target, opening: Opening) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let control = Arc::new(Control {
            target,
            endpoints: Mutex::new(Some(opening.endpoints)),
            commands: Mutex::new(Vec::with_capacity(2)),
            settled: AtomicBool::new(false),
        });
        let (requests, receiver) = mpsc::channel(1);
        let (stop, stopping) = watch::channel(false);
        let (report, failure) = watch::channel(None);
        let (ready, received) = oneshot::channel();
        let owner = control.clone();
        let handle = std::thread::Builder::new()
            .name("blent-adb-mapping".into())
            .spawn(move || {
                runtime.block_on(worker::run(owner, receiver, stopping, ready, report))
            })?;
        let completion = opening.admission.completion();
        opening.admission.retain(Retiring {
            thread: blent_config::retirement_work::Thread::new(handle),
            control: control.clone(),
        });
        let lease = Self {
            control,
            requests,
            stop,
            failure,
            completion,
        };
        received
            .await
            .map_err(|_| anyhow::anyhow!("ADB mapping creator stopped; ownership retained"))?
            .map_err(anyhow::Error::msg)?;
        Ok(lease)
    }

    pub fn target(&self) -> &Target {
        &self.control.target
    }

    pub async fn shell(&self, input: String) -> Result<()> {
        ensure!(input.len() <= 4096, "ADB media invitation exceeds 4 KiB");
        let (result, received) = oneshot::channel();
        let mut request = Some(Invocation { input, result });
        let mut accepted = Err(anyhow::anyhow!("ADB mapping is retiring"));
        // Admission and cutoff share the watch value's write lock. No native
        // effect or await occurs under it, and full queues reject immediately.
        self.stop.send_if_modified(|stopping| {
            if !*stopping {
                accepted = self
                    .requests
                    .try_send(request.take().unwrap())
                    .map_err(|_| anyhow::anyhow!("ADB mapping invitation queue unavailable"));
            }
            false
        });
        accepted?;
        received
            .await
            .map_err(|_| anyhow::anyhow!("ADB mapping invitation cancelled"))?
            .map_err(anyhow::Error::msg)
    }

    pub async fn close(&self) -> Result<()> {
        self.stop.send_replace(true);
        let mut failure = self.failure.clone();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !self.completion.is_complete() {
                if let Some(error) = failure.borrow_and_update().clone() {
                    anyhow::bail!("{error}; original ADB mapping cleanup remains retained");
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Ok(())
        })
        .await
        .map_err(|_| anyhow::anyhow!("ADB mapping cleanup pending; original ownership retained"))?
    }
}

#[cfg(test)]
mod tests;

impl Drop for Lease {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}

struct Retiring {
    thread: blent_config::retirement_work::Thread,
    control: Arc<Control>,
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        self.thread.poll_complete() && self.control.settled.load(Ordering::Acquire)
    }
}
