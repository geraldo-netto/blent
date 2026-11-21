//! Fill only unspecified H.264 color fields before FFmpeg decodes them.
//! One borrowed packet; optional rewritten packet <= MAX_PACKET and one SPS <=4096 bytes.
use anyhow::{ensure, Result};
use h264_reader::nal::{sps::SeqParameterSet, Nal, RefNal, WritableNal};
use std::borrow::Cow;

const MAX_SPS: usize = 4096;

fn sps(bytes: &[u8]) -> Result<Vec<u8>> {
    ensure!(bytes.len() <= MAX_SPS, "Camera SPS exceeds metadata budget");
    let nal = RefNal::new(bytes, &[], true);
    let header = nal
        .header()
        .map_err(|error| anyhow::anyhow!("Invalid camera NAL: {error:?}"))?;
    let mut parsed = SeqParameterSet::from_bits(nal.rbsp_bits())
        .map_err(|error| anyhow::anyhow!("Invalid camera SPS: {error:?}"))?;
    defaults(&mut parsed);
    let mut output = [0u8; MAX_SPS];
    let mut writer = std::io::Cursor::new(output.as_mut_slice());
    parsed.write_with_header(header, &mut writer)?;
    let length = writer.position() as usize;
    Ok(output[..length].to_vec())
}

fn defaults(parsed: &mut SeqParameterSet) {
    use h264_reader::nal::sps::{ColourDescription, VideoFormat, VideoSignalType};
    let signal = parsed
        .vui_parameters
        .get_or_insert_with(Default::default)
        .video_signal_type
        .get_or_insert_with(|| VideoSignalType {
            video_format: VideoFormat::Unspecified,
            ..Default::default()
        });
    let description = signal.colour_description.get_or_insert(ColourDescription {
        colour_primaries: 2,
        transfer_characteristics: 2,
        matrix_coefficients: 2,
    });
    unspecified(&mut description.colour_primaries, 6);
    unspecified(&mut description.transfer_characteristics, 1);
    unspecified(&mut description.matrix_coefficients, 6);
}

fn unspecified(value: &mut u8, default: u8) {
    if *value == 2 {
        *value = default;
    }
}

fn append(output: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
    ensure!(
        output.len() + bytes.len() <= super::protocol::MAX_PACKET,
        "Normalized camera packet exceeds budget"
    );
    output.try_reserve_exact(bytes.len())?;
    output.extend_from_slice(bytes);
    Ok(())
}

pub(super) fn normalize(packet: &[u8]) -> Result<Cow<'_, [u8]>> {
    ensure!(
        packet.len() <= super::protocol::MAX_PACKET,
        "Camera packet exceeds budget"
    );
    let mut starts = packet
        .windows(3)
        .enumerate()
        .filter_map(|(index, bytes)| (bytes == [0, 0, 1]).then_some(index + 3))
        .peekable();
    let mut output = Vec::new();
    let mut copied = 0;
    while let Some(start) = starts.next() {
        let end = starts.peek().map_or(packet.len(), |next| next - 3);
        let nal = &packet[start..end];
        if nal.first().is_some_and(|header| header & 31 == 7) {
            append(&mut output, &packet[copied..start])?;
            append(&mut output, &sps(nal)?)?;
            copied = end;
        }
    }
    if copied == 0 {
        return Ok(Cow::Borrowed(packet));
    }
    append(&mut output, &packet[copied..])?;
    Ok(Cow::Owned(output))
}

#[cfg(test)]
mod tests;
