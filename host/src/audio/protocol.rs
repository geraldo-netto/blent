use anyhow::{ensure, Result};
use blent_config::audio::{AudioCapabilities, AudioGrant, Direction, Processing};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub(super) async fn accept(
    listener: &TcpListener,
    token: &str,
    direction: Direction,
) -> Result<(TcpStream, AudioCapabilities, Processing, String, bool)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    loop {
        let (mut socket, _) = tokio::time::timeout_at(deadline, listener.accept()).await??;
        socket.set_nodelay(true)?;
        let request_deadline = (tokio::time::Instant::now() + Duration::from_secs(2)).min(deadline);
        let result =
            tokio::time::timeout_at(request_deadline, request(&mut socket, token, direction)).await;
        if let Ok(Ok((caps, mode, detail, clocked))) = result {
            return Ok((socket, caps, mode, detail, clocked));
        }
        ensure!(
            tokio::time::Instant::now() < deadline,
            "Audio consent timed out"
        );
    }
}
async fn request(
    socket: &mut TcpStream,
    token: &str,
    direction: Direction,
) -> Result<(AudioCapabilities, Processing, String, bool)> {
    let mut bytes = [0; 76];
    socket.read_exact(&mut bytes).await?;
    ensure!(
        matches!(&bytes[..8], b"BLAUREQ1" | b"BLAUREQ2"),
        "Unknown audio request"
    );
    let clocked = &bytes[..8] == b"BLAUREQ2";
    ensure!(
        blent_config::credentials::token_matches(token, std::str::from_utf8(&bytes[8..72])?),
        "Audio request authentication failed"
    );
    ensure!(
        bytes[72] <= 7
            && bytes[73] <= 1
            && bytes[74] == direction as u8
            && matches!(bytes[75], 1 | 2),
        "Invalid audio capabilities"
    );
    let capabilities = AudioCapabilities {
        microphone: direction == Direction::Microphone,
        speakers: direction == Direction::Speakers,
        speech: bytes[72] & 1 != 0,
        raw: bytes[72] & 2 != 0,
        background: bytes[72] & 4 != 0,
    };
    let detail = if direction == Direction::Speakers {
        "Speaker playback prepared; echo control belongs to the microphone session."
    } else if bytes[73] != 0 {
        "Speech AEC enabled; acoustic effectiveness is device-dependent."
    } else {
        "AEC unavailable or raw processing selected."
    };
    let processing = if bytes[75] == 1 {
        Processing::Speech
    } else {
        Processing::Raw
    };
    Ok((
        capabilities,
        processing,
        format!("Effective {processing:?}. {detail}"),
        clocked,
    ))
}

pub(super) fn captured(bytes: &[u8]) -> Result<blent_config::audio::PcmBlock> {
    ensure!(
        matches!(bytes.len(), 1921 | 1945),
        "invalid native speaker packet length"
    );
    ensure!(bytes[0] <= 1, "invalid native speaker discontinuity");
    let pcm = bytes.len() - 1920;
    let mut block =
        blent_config::audio::PcmBlock::from_le_bytes(Direction::Speakers, &bytes[pcm..])?;
    block.discontinuity = bytes[0] != 0;
    if pcm > 1 {
        block.clock = blent_config::audio::ClockSample::decode(&bytes[1..pcm])?;
    }
    Ok(block)
}
pub(super) async fn grant(socket: &mut TcpStream, ticket: &str, grant: &AudioGrant) -> Result<()> {
    let bytes = [ticket.as_bytes(), grant.hello().as_slice()].concat();
    tokio::time::timeout(Duration::from_secs(2), socket.write_all(&bytes)).await??;
    Ok(())
}

/// Fixed staging owned by one connection. A borrow prevents overwrite while the
/// native writer still consumes the packet. Partial I/O retires the connection.
pub(super) struct Staging {
    bytes: [u8; 1 + blent_config::audio::CLOCK_BYTES + blent_config::audio::MAX_SAMPLES * 2],
}
impl Staging {
    pub(super) fn new() -> Self {
        Self {
            bytes: [0; 1 + blent_config::audio::CLOCK_BYTES + blent_config::audio::MAX_SAMPLES * 2],
        }
    }
    /// One deadline belongs around this whole operation, including partial reads.
    pub(super) async fn packet(
        &mut self,
        reader: &mut (impl tokio::io::AsyncRead + Unpin),
        frames: &mut blent_config::audio::FrameReader,
    ) -> Result<&[u8]> {
        let mut header = [0; blent_config::audio::FRAME_HEADER_BYTES];
        reader.read_exact(&mut header).await?;
        let size = frames.payload_bytes(&header)?;
        reader.read_exact(&mut self.bytes[1..1 + size]).await?;
        let view = frames.validate_parts(&header, &self.bytes[1..1 + size])?;
        self.bytes[0] = u8::from(view.discontinuity);
        Ok(&self.bytes[..1 + size])
    }
}

#[cfg(test)]
pub(super) async fn packet(
    reader: &mut (impl tokio::io::AsyncRead + Unpin),
    frames: &mut blent_config::audio::FrameReader,
) -> Result<Vec<u8>> {
    Ok(Staging::new().packet(reader, frames).await?.to_vec())
}
