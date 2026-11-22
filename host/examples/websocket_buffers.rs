//! T1738: bounded loopback WebSocket buffer comparison; never opens tablet devices.
//! Each task owns one listener and both socket futures. try_join drops the other
//! future on failure; collection joins every task before reporting or returning.
use anyhow::{ensure, Context, Result};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::protocol::WebSocketConfig, tungstenite::Message};

type Socket = tokio_tungstenite::WebSocketStream<TcpStream>;

#[derive(Parser, Clone, Copy, serde::Serialize)]
struct Options {
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(4..=128))]
    buffer_kib: u32,
    #[arg(long, default_value_t = 200, value_parser = clap::value_parser!(u32).range(1..=65536))]
    payload: u32,
    #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u32).range(1..=16))]
    clients: u32,
    #[arg(long, default_value_t = 2000, value_parser = clap::value_parser!(u32).range(1..=10000))]
    frames: u32,
    #[arg(long)]
    burst: bool,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let options = Options::parse();
    let started = Instant::now();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..options.clients {
        tasks.spawn(pair(options));
    }
    let mut samples = collect(tasks).await?;
    let seconds = started.elapsed().as_secs_f64();
    samples.sort_unstable();
    println!(
        "{}",
        serde_json::json!({
            "options": options, "frames": options.frames * options.clients,
            "seconds": seconds,
            "payload_mib_per_second": f64::from(options.payload) * f64::from(options.frames) * f64::from(options.clients) / seconds / 1048576.0,
            "roundtrip_p50_ns": samples.get(samples.len() / 2),
            "roundtrip_p95_ns": samples.get(samples.len() * 95 / 100),
            "peak_rss_kib": peak_rss()?,
            "server_read_capacity_bytes": options.buffer_kib * 1024 * options.clients,
        })
    );
    Ok(())
}

async fn collect(mut tasks: tokio::task::JoinSet<Result<Vec<u64>>>) -> Result<Vec<u64>> {
    let mut samples = Vec::new();
    let mut failure = None;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(values)) => samples.extend(values),
            Ok(Err(error)) => failure = Some(error),
            Err(error) => failure = Some(error.into()),
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(samples),
    }
}

async fn pair(options: Options) -> Result<Vec<u64>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (_, samples) = tokio::try_join!(echo(listener, options), client(address, options))?;
    Ok(samples)
}

async fn echo(listener: TcpListener, options: Options) -> Result<()> {
    let (stream, _) = listener.accept().await?;
    stream.set_nodelay(true)?;
    let config = WebSocketConfig::default()
        .read_buffer_size(options.buffer_kib as usize * 1024)
        .max_message_size(Some(65536))
        .max_frame_size(Some(65536));
    let mut socket = tokio_tungstenite::accept_async_with_config(stream, Some(config)).await?;
    for _ in 0..options.frames {
        let frame = socket.next().await.context("peer ended early")??;
        ensure!(
            frame.is_binary() && frame.len() == options.payload as usize,
            "unexpected frame"
        );
        socket.send(frame).await?;
    }
    Ok(())
}

async fn client(address: std::net::SocketAddr, options: Options) -> Result<Vec<u64>> {
    let stream = TcpStream::connect(address).await?;
    stream.set_nodelay(true)?;
    let config = WebSocketConfig::default().read_buffer_size(4096);
    let (socket, _) =
        tokio_tungstenite::client_async_with_config("ws://localhost", stream, Some(config)).await?;
    let payload = Message::binary(vec![17; options.payload as usize]);
    if options.burst {
        burst(socket, payload, options.frames).await?;
        Ok(Vec::new())
    } else {
        interactive(socket, payload, options.frames).await
    }
}

async fn interactive(mut socket: Socket, payload: Message, frames: u32) -> Result<Vec<u64>> {
    let mut samples = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        let started = Instant::now();
        socket.send(payload.clone()).await?;
        let reply = socket.next().await.context("echo ended early")??;
        ensure!(reply == payload, "echo changed payload");
        samples.push(u64::try_from(started.elapsed().as_nanos())?);
    }
    Ok(samples)
}

async fn burst(socket: Socket, payload: Message, frames: u32) -> Result<()> {
    let (mut sent, mut received) = socket.split();
    let writer = async {
        for _ in 0..frames {
            sent.send(payload.clone()).await?;
        }
        Ok::<_, anyhow::Error>(())
    };
    let reader = async {
        for _ in 0..frames {
            let reply = received.next().await.context("echo ended early")??;
            ensure!(reply == payload, "echo changed payload");
        }
        Ok::<_, anyhow::Error>(())
    };
    tokio::try_join!(writer, reader)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn peak_rss() -> Result<Option<i64>> {
    use std::io::Read;
    // VmHWM belongs to this executable's address space; getrusage can retain
    // the launcher's pre-exec high-water mark and conceal small differences.
    let mut status = String::new();
    std::fs::File::open("/proc/self/status")?
        .take(32768)
        .read_to_string(&mut status)?;
    let row = status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .context("kernel did not report VmHWM")?;
    let fields: Vec<_> = row.split_whitespace().collect();
    ensure!(
        fields.len() == 3 && fields[2] == "kB",
        "invalid VmHWM units"
    );
    Ok(Some(fields[1].parse()?))
}

#[cfg(not(target_os = "linux"))]
fn peak_rss() -> Result<Option<i64>> {
    // No native memory measurement on unqualified platforms.
    Ok(None)
}
