use super::*;
fn grant(clocked: bool) -> AudioGrant {
    let mut grant = AudioGrant::new(
        AudioProfile::new(Direction::Speakers),
        1,
        crate::credentials::tests::entropy,
    )
    .unwrap();
    grant.clocked = clocked;
    grant
}
#[test]
fn t818_views_validate_before_commit_and_preserve_endian_and_clock() {
    for clocked in [false, true] {
        let grant = grant(clocked);
        let samples = [i16::MIN; MAX_SAMPLES];
        let clock = Some(ClockSample {
            epoch: 1,
            frames: 480,
            nanos: 1,
        });
        let packet = grant.encode_with_clock(0, 1, &samples, clock).unwrap();
        let mut reader = grant.authenticate(&grant.hello()).unwrap();
        for length in 0..packet.len() - FRAME_HEADER_BYTES {
            assert!(reader
                .validate_parts(&packet[..28], &packet[28..28 + length])
                .is_err());
        }
        for length in 0..28 {
            assert!(reader
                .validate_parts(&packet[..length], &packet[28..])
                .is_err());
        }
        let view = reader.validate_parts(&packet[..28], &packet[28..]).unwrap();
        assert_eq!(
            view.pcm.as_ptr(),
            packet[28 + if clocked { CLOCK_BYTES } else { 0 }..].as_ptr()
        );
        assert_eq!(&view.pcm[..2], &i16::MIN.to_le_bytes());
        assert_eq!(view.clock, if clocked { clock } else { None });
        assert!(!view.discontinuity);
        assert!(reader.validate_parts(&packet[..28], &packet[28..]).is_err());
        let gap = grant.encode_with_clock(2, 2, &samples, clock).unwrap();
        assert!(
            reader
                .validate_parts(&gap[..28], &gap[28..])
                .unwrap()
                .discontinuity
        );
    }
}
#[test]
fn t818_reusable_writer_is_atomic_on_invalid_input() {
    let grant = grant(true);
    let mut writer = FrameWriter::new(grant.clone());
    let mut bytes = vec![255; 28 + CLOCK_BYTES + MAX_SAMPLES * 2];
    let pointer = bytes.as_ptr();
    let mut block = PcmBlock::from_le_bytes(Direction::Speakers, &[1; MAX_SAMPLES * 2]).unwrap();
    writer.encode_into(&block, 1, &mut bytes).unwrap();
    assert_eq!(bytes, grant.encode(0, 1, block.samples()).unwrap());
    let previous = bytes.clone();
    assert!(writer.encode_into(&block, 1, &mut bytes).is_err());
    block.clock = Some(ClockSample {
        epoch: 0,
        frames: 1,
        nanos: 1,
    });
    assert!(writer.encode_into(&block, 2, &mut bytes).is_err());
    assert_eq!(bytes, previous);
    block.clock = None;
    block.discontinuity = true;
    writer.encode_into(&block, 2, &mut bytes).unwrap();
    assert_eq!(bytes, grant.encode(2, 2, block.samples()).unwrap());
    assert_eq!(bytes.as_ptr(), pointer);
}
