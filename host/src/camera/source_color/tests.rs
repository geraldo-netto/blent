use super::*;
const SPS: &[u8] = include_bytes!("../../../../testdata/camera-color-sps.h264");

fn parse(packet: &[u8]) -> SeqParameterSet {
    SeqParameterSet::from_bits(RefNal::new(&packet[3..], &[], true).rbsp_bits()).unwrap()
}

#[test]
fn t1461_unspecified_fields_get_defaults_and_signalled_fields_survive() {
    let input = parse(SPS);
    for primaries in [1, 2, 5, 6, 9] {
        for transfer in [1, 2, 6, 13] {
            let mut parsed = input.clone();
            let signal = parsed
                .vui_parameters
                .get_or_insert_with(Default::default)
                .video_signal_type
                .get_or_insert_with(Default::default);
            signal.video_full_range_flag = true;
            signal.colour_description = Some(h264_reader::nal::sps::ColourDescription {
                colour_primaries: primaries,
                transfer_characteristics: transfer,
                matrix_coefficients: 1,
            });
            let mut packet = vec![0, 0, 1];
            parsed
                .write_with_header(
                    RefNal::new(&SPS[3..], &[], true).header().unwrap(),
                    &mut packet,
                )
                .unwrap();
            let normalized = normalize(&packet).unwrap();
            let output = parse(&normalized);
            let signal = output.vui_parameters.unwrap().video_signal_type.unwrap();
            let color = signal.colour_description.unwrap();
            assert_eq!(
                color.colour_primaries,
                if primaries == 2 { 6 } else { primaries }
            );
            assert_eq!(
                color.transfer_characteristics,
                if transfer == 2 { 1 } else { transfer }
            );
            assert_eq!(color.matrix_coefficients, 1);
            assert!(signal.video_full_range_flag);
        }
    }
}

#[test]
fn t1461_packet_rewrite_preserves_other_nals_and_is_idempotent() {
    let mut packet = SPS.to_vec();
    let suffix = [0, 0, 0, 1, 0x68, 1, 2, 3, 0, 0, 1, 0x65, 4, 5];
    packet.extend_from_slice(&suffix);
    let output = normalize(&packet).unwrap();
    assert!(output.ends_with(&suffix[1..]));
    assert_eq!(normalize(&output).unwrap(), output);
    assert!(matches!(normalize(&suffix).unwrap(), Cow::Borrowed(_)));
    let defaults = parse(&normalize(SPS).unwrap())
        .vui_parameters
        .unwrap()
        .video_signal_type
        .unwrap()
        .colour_description
        .unwrap();
    assert_eq!(
        (
            defaults.colour_primaries,
            defaults.transfer_characteristics,
            defaults.matrix_coefficients
        ),
        (6, 1, 6)
    );
}

#[test]
fn t1461_malformed_and_oversized_metadata_stays_bounded() {
    assert!(normalize(&vec![0; super::super::protocol::MAX_PACKET + 1]).is_err());
    assert!(sps(&vec![0x67; MAX_SPS + 1]).is_err());
    for length in 4..SPS.len() - 1 {
        assert!(normalize(&SPS[..length]).is_err());
    }
    for bit in 0..(SPS.len() * 8) {
        let mut packet = SPS.to_vec();
        packet[bit / 8] ^= 1 << (bit % 8);
        if let Ok(output) = normalize(&packet) {
            assert!(output.len() <= super::super::protocol::MAX_PACKET);
        }
    }
    let mut output = vec![0; super::super::protocol::MAX_PACKET];
    assert!(append(&mut output, &[1]).is_err());
    assert_eq!(output.len(), super::super::protocol::MAX_PACKET);
}
