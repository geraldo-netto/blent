//! Linux audio transport and isolated PipeWire adapter.
use crate::{audio_control::Report, camera_control::cancelled};
use anyhow::{ensure, Context, Result};
use blent_config::audio::connection::{Connection, Target};
use blent_config::audio::{AudioOptions, AudioSession, AudioState, Direction};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
#[cfg(test)]
use tokio::process::Command;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::watch,
};
mod adb;
mod process;
mod protocol;
#[cfg(feature = "accessory-transport")]
pub(crate) mod usb;

/// Implemented adapters only; physical qualification remains separate. Both
/// independent audio directions share the validated original USB media adapter.
pub const fn transport_capabilities(
    _direction: Direction,
) -> blent_config::audio::connection::Transports {
    blent_config::audio::connection::Transports {
        adb: true,
        usb: cfg!(feature = "accessory-transport"),
    }
}

pub async fn run(
    options: AudioOptions,
    mut stop: watch::Receiver<bool>,
    report: Report,
) -> Result<()> {
    options.validate()?;
    if *stop.borrow() {
        return Ok(());
    }
    ensure!(
        transport_capabilities(options.profile.direction).supports(&options.target()?),
        "direct-USB audio is not implemented"
    );
    let helper = helper()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let token = blent_config::credentials::random_token()?;
    let bridge = connection(&options, &listener, &token, &report).await?;
    let session = Session {
        listener: &listener,
        connection: bridge.as_ref(),
        helper: &helper,
        options: &options,
        token: &token,
    };
    let result = session.serve(&report, &mut stop).await;
    let cleanup = bridge.close().await;
    drop(bridge);
    report.wait_native().await;
    result.and(cleanup)
}

async fn connection(
    options: &AudioOptions,
    listener: &TcpListener,
    _token: &str,
    _report: &Report,
) -> Result<Box<dyn Connection<blent_config::audio::AudioProfile>>> {
    match options.target()? {
        Target::Adb(serial) => Ok(Box::new(
            adb::Bridge::create(executable("adb")?, serial, listener.local_addr()?.port()).await?,
        )),
        #[cfg(feature = "accessory-transport")]
        Target::Usb(_) => {
            use std::os::fd::AsFd;
            let retained = std::net::TcpListener::from(listener.as_fd().try_clone_to_owned()?);
            Ok(Box::new(usb::native(
                options,
                _token,
                retained,
                crate::broker::linux::client::Launch::from_environment()?,
                _report,
            )?))
        }
        #[cfg(not(feature = "accessory-transport"))]
        Target::Usb(_) => anyhow::bail!("direct-USB audio requires accessory-transport support"),
    }
}

fn executable(name: &str) -> Result<PathBuf> {
    blent_config::linux::programs::find_in(name, &std::env::var_os("PATH").unwrap_or_default())
        .with_context(|| format!("Audio requires {name}"))
}
fn helper() -> Result<PathBuf> {
    let current = std::env::current_exe()?;
    let sibling = current
        .parent()
        .context("missing executable directory")?
        .join("blent-audio");
    if sibling.is_file() {
        return Ok(sibling);
    }
    executable("blent-audio")
}

/// Linux session orchestration depends on the independent typed connection port;
/// local device creation remains in the process adapter. It never selects a new
/// peer, consumes a new invitation or reconnects after original authority loss.
pub(crate) struct Session<'a> {
    pub(crate) listener: &'a TcpListener,
    pub(crate) connection: &'a dyn Connection<blent_config::audio::AudioProfile>,
    pub(crate) helper: &'a Path,
    pub(crate) options: &'a AudioOptions,
    pub(crate) token: &'a str,
}
/// Immutable result of this stream's authenticated negotiation. Moving it
/// transfers the original grant; it contains no device or process ownership.
struct NegotiatedStream {
    grant: blent_config::audio::AudioGrant,
    processing: String,
    clocked: bool,
}
impl Session<'_> {
    pub(crate) async fn serve(
        &self,
        report: &Report,
        stop: &mut watch::Receiver<bool>,
    ) -> Result<()> {
        self.serve_with(report, stop, &process::spawn_with_clock)
            .await
    }
    // Internal Linux process boundary seam: tests can attach a real dependent
    // lease to OwnedChild without replacing retirement or connection semantics.
    async fn serve_with(
        &self,
        report: &Report,
        stop: &mut watch::Receiver<bool>,
        spawn: &process::Start<'_>,
    ) -> Result<()> {
        self.options.validate()?;
        ensure!(
            blent_config::connection::media::valid_credential(self.token),
            "invalid audio session credential"
        );
        if *stop.borrow() {
            return Ok(());
        }
        let setup = async {
            self.connection
                .invite(self.token, &self.options.profile)
                .await?;
            protocol::accept(self.listener, self.token, self.options.profile.direction).await
        };
        let accepted = tokio::select! {
            biased;
            _ = cancelled(stop) => return Ok(()),
            result = self.connection.monitor() => return lost(result),
            result = setup => result?,
        };
        self.stream(accepted, report, stop, spawn).await
    }
    async fn stream(
        &self,
        accepted: protocol::Accepted,
        report: &Report,
        stop: &mut watch::Receiver<bool>,
        spawn: &process::Start<'_>,
    ) -> Result<()> {
        let (mut socket, capabilities, mode, processing, clocked) = accepted;
        let mut session = AudioSession::new(self.options.profile.direction);
        let mut profile = self.options.profile;
        profile.processing = mode;
        let grant = session.start_with_clock(profile, capabilities, true, 0, clocked)?;
        let mut child = spawn(self.helper, self.options.profile, self.token, clocked)?;
        report.retain(child.completion())?;
        let stream = self.transfer(
            &mut socket,
            &mut child,
            &mut session,
            NegotiatedStream {
                grant,
                processing,
                clocked,
            },
            report,
        );
        let result = tokio::select! {
            biased;
            _ = cancelled(stop) => Ok(()),
            result = self.connection.monitor() => lost(result),
            result = stream => result,
        };
        let retired = process::retire(child).await;
        ensure!(
            retired.completion.is_complete(),
            "Audio process retirement incomplete; capacity retained"
        );
        result
    }
    async fn transfer(
        &self,
        socket: &mut TcpStream,
        child: &mut blent_config::commands::OwnedChild,
        session: &mut AudioSession,
        negotiated: NegotiatedStream,
        report: &Report,
    ) -> Result<()> {
        let NegotiatedStream {
            grant,
            processing,
            clocked,
        } = negotiated;
        let started = std::time::Instant::now();
        process::ready(child)
            .await
            .context("PipeWire unavailable or virtual audio device setup failed")?;
        protocol::grant(socket, self.token, &grant).await?;
        session.connected(
            grant.generation(),
            &grant.hello(),
            started.elapsed().as_millis() as u64,
        )?;
        report.update(
            AudioState::Streaming,
            format!(
                "Blent {:?} available. {processing}",
                self.options.profile.direction
            ),
        );
        match self.options.profile.direction {
            Direction::Microphone => {
                let mut reader = grant.authenticate(&grant.hello())?;
                microphone(socket, child, &mut reader).await
            }
            Direction::Speakers => speakers_with_clock(socket, child, grant, clocked).await,
        }
    }
}
fn lost(result: Result<()>) -> Result<()> {
    result?;
    anyhow::bail!("original audio connection ended")
}

async fn microphone(
    socket: &mut TcpStream,
    child: &mut tokio::process::Child,
    reader: &mut blent_config::audio::FrameReader,
) -> Result<()> {
    let input = child.stdin.as_mut().context("missing audio helper input")?;
    let mut staging = protocol::Staging::new();
    loop {
        let packet =
            tokio::time::timeout(Duration::from_millis(250), staging.packet(socket, reader))
                .await??;
        tokio::time::timeout(Duration::from_millis(250), input.write_all(packet)).await??;
    }
}

#[cfg(test)]
async fn speakers(
    socket: &mut TcpStream,
    child: &mut tokio::process::Child,
    grant: blent_config::audio::AudioGrant,
) -> Result<()> {
    speakers_with_clock(socket, child, grant, false).await
}

async fn speakers_with_clock(
    socket: &mut TcpStream,
    child: &mut tokio::process::Child,
    grant: blent_config::audio::AudioGrant,
    clocked: bool,
) -> Result<()> {
    use tokio::io::AsyncReadExt;
    let output = child
        .stdout
        .as_mut()
        .context("missing audio helper output")?;
    let mut writer = blent_config::audio::FrameWriter::new(grant);
    let epoch = std::time::Instant::now();
    let mut bytes = vec![0; if clocked { 1945 } else { 1921 }];
    let mut packet = Vec::with_capacity(blent_config::audio::FRAME_HEADER_BYTES + 1944);
    loop {
        tokio::time::timeout(Duration::from_millis(250), output.read_exact(&mut bytes)).await??;
        let block = protocol::captured(&bytes)?;
        writer.encode_into(&block, epoch.elapsed().as_micros() as u64, &mut packet)?;
        tokio::time::timeout(Duration::from_millis(250), socket.write_all(&packet)).await??;
    }
}

/// CLI shares the exact GUI pipeline and retires only its owned direction.
pub async fn run_cli(options: AudioOptions) -> Result<()> {
    let (stop, stopped) = watch::channel(false);
    let (report, _) = watch::channel(Default::default());
    let mut work = std::pin::pin!(run(options, stopped, Report::new(report)));
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    crate::control_outcome::with_signals(work.as_mut(), &stop, tokio::signal::ctrl_c(), async {
        terminate.recv().await;
    })
    .await
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod connection_tests;

#[cfg(test)]
pub(crate) mod session_tests;
