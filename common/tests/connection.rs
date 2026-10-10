//! T846: shared cross-language vectors and bounded malformed-envelope properties.
use blent_config::connection::wire::*;

fn valid() -> Header {
    Header {
        kind: Kind::Data,
        channel: Channel::Control,
        session: 1,
        generation: 2,
        sequence: 0,
        length: 16,
        credit: 0,
    }
}
fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn t846_shared_connection_vectors() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../testdata/connection-v1.json")).unwrap();
    for vector in vectors.as_array().unwrap() {
        let input = bytes(vector["hex"].as_str().unwrap());
        let result = Header::decode(&input);
        assert_eq!(
            result.is_ok(),
            vector["valid"].as_bool().unwrap(),
            "{}",
            vector["name"]
        );
        if let Ok(header) = result {
            assert_eq!(header.session, vector["session"].as_u64().unwrap());
            assert_eq!(header.generation, vector["generation"].as_u64().unwrap());
            assert_eq!(header.sequence, vector["sequence"].as_u64().unwrap());
            assert_eq!(header.length as u64, vector["length"].as_u64().unwrap());
            assert_eq!(header.credit as u64, vector["credit"].as_u64().unwrap());
            assert_eq!(header.kind as u64, vector["kind"].as_u64().unwrap());
            assert_eq!(header.channel as u64, vector["channel"].as_u64().unwrap());
            let mut output = [0; HEADER_BYTES];
            header.encode(&mut output).unwrap();
            assert_eq!(output.as_slice(), input);
        }
    }
}
#[test]
fn t846_header_extents_and_atomic_encode_rejection() {
    for length in 0..=80 {
        let mut output = vec![0xa5; length];
        if length != HEADER_BYTES {
            assert!(valid().encode(&mut output).is_err());
            assert_eq!(output, vec![0xa5; length]);
            assert!(Header::decode(&output).is_err());
        }
    }
    let mut output = [0xa5; HEADER_BYTES];
    let mut bad = valid();
    bad.session = 0;
    assert!(bad.encode(&mut output).is_err());
    assert_eq!(output, [0xa5; HEADER_BYTES]);
    for value in 0..=255 {
        assert_eq!(
            Channel::try_from(value).is_ok(),
            (1..=CHANNELS).contains(&value)
        );
        assert_eq!(Kind::try_from(value).is_ok(), value <= 3);
    }
}
#[test]
fn t846_bounded_header_mutations_never_allocate_or_panic() {
    let mut seed = 846u64;
    let mut original = [0; HEADER_BYTES];
    valid().encode(&mut original).unwrap();
    for _ in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut input = original;
        input[(seed as usize) % HEADER_BYTES] ^= (seed >> 32) as u8;
        if let Ok(header) = Header::decode(&input) {
            let mut output = [0; HEADER_BYTES];
            header.encode(&mut output).unwrap();
            assert_eq!(input, output);
            assert!(header.length <= MAX_PAYLOAD && header.credit <= MAX_CREDIT);
        }
    }
}

#[test]
fn t846_shared_credit_and_retirement_trace() {
    use blent_config::connection::flow::Flow;
    let mut flow = Flow::new(7, 9, Channel::Control, 64).unwrap();
    let steps: serde_json::Value =
        serde_json::from_str(include_str!("../../testdata/connection-flow-v1.json")).unwrap();
    for (index, step) in steps.as_array().unwrap().iter().enumerate() {
        let before = flow.clone();
        let result = flow_step(&mut flow, step);
        assert_eq!(
            result.is_ok(),
            step["ok"].as_bool().unwrap(),
            "step {index}: {step} {result:?}"
        );
        if result.is_err() {
            assert_eq!(flow, before, "rejection changed state at {index}");
        }
    }
}
fn flow_step(
    flow: &mut blent_config::connection::flow::Flow,
    step: &serde_json::Value,
) -> anyhow::Result<()> {
    let length = step["length"].as_u64().unwrap() as u32;
    if step["direction"] == "release" {
        return flow.release(length);
    }
    let header = Header {
        kind: Kind::try_from(step["kind"].as_u64().unwrap() as u8)?,
        channel: Channel::try_from(step["channel"].as_u64().unwrap() as u8)?,
        session: step["session"].as_u64().unwrap(),
        generation: step["generation"].as_u64().unwrap(),
        sequence: step["sequence"].as_u64().unwrap(),
        length,
        credit: step["credit"].as_u64().unwrap() as u32,
    };
    if step["direction"] == "receive" {
        return flow.receive(header);
    }
    let sent = flow.send(header.kind, header.length, header.credit)?;
    assert_eq!(sent, header);
    Ok(())
}
#[test]
fn t846_credit_boundaries_and_independent_channel_retirement() {
    use blent_config::connection::flow::Flow;
    for capacity in [0, MAX_CREDIT + 1, u32::MAX] {
        assert!(Flow::new(1, 1, Channel::Control, capacity).is_err());
    }
    for session in [0, MAX_COUNTER + 1, u64::MAX] {
        assert!(Flow::new(session, 1, Channel::Control, 1).is_err());
    }
    for generation in [0, MAX_COUNTER + 1, u64::MAX] {
        assert!(Flow::new(1, generation, Channel::Control, 1).is_err());
    }
    let mut control = Flow::new(1, 1, Channel::Control, MAX_CREDIT).unwrap();
    let mut video = Flow::new(1, 1, Channel::Display, 1).unwrap();
    video.send(Kind::Reset, 0, 0).unwrap();
    assert!(video.send(Kind::Credit, 0, 1).is_err());
    control.send(Kind::Credit, 0, MAX_CREDIT).unwrap();
    let mut header = Header {
        kind: Kind::Data,
        channel: Channel::Control,
        session: 1,
        generation: 1,
        sequence: 0,
        length: 1,
        credit: 0,
    };
    control.receive(header).unwrap();
    assert!(control.send(Kind::Credit, 0, 1).is_err());
    control.release(1).unwrap();
    control.send(Kind::Credit, 0, 1).unwrap();
    header.kind = Kind::Close;
    header.length = 0;
    header.sequence = 1;
    control.receive(header).unwrap();
    assert!(control.receive(header).is_err());
    let mut replacement = Flow::new(1, 2, Channel::Control, 1).unwrap();
    assert!(replacement.receive(header).is_err());
    assert!(replacement.release(u32::MAX).is_err());
}

#[test]
fn t846_fair_queue_skips_stalls_and_borrows_buffers() {
    use blent_config::connection::queue::Scheduler;
    let mut queue = Scheduler::new(64, 4, 4).unwrap();
    let control = vec![1, 2, 3, 4, 5, 6];
    let address = control.as_ptr();
    queue.offer(Channel::Control, control).unwrap();
    queue.offer(Channel::Display, vec![9; 16]).unwrap();
    queue.offer(Channel::Microphone, vec![7; 3]).unwrap();
    let chunk = queue.next(&[64, 0, 0, 64, 0, 0]).unwrap().unwrap();
    assert_eq!(chunk.channel, Channel::Control);
    assert_eq!(chunk.bytes, &[1, 2, 3, 4]);
    assert_eq!(chunk.bytes.as_ptr(), address); // no staging copy
    assert!(queue.next(&[64; CHANNELS as usize]).unwrap().is_none());
    assert!(queue.clear(Channel::Control).is_err());
    queue.clear(Channel::Camera).unwrap();
    queue.finish(true).unwrap();
    let chunk = queue.next(&[64, 0, 0, 64, 0, 0]).unwrap().unwrap();
    assert_eq!(chunk.channel, Channel::Microphone);
    assert_eq!(chunk.bytes, &[7; 3]);
    queue.finish(false).unwrap(); // cancellation does not drop bytes
    let chunk = queue.next(&[64, 0, 0, 64, 0, 0]).unwrap().unwrap();
    assert_eq!(chunk.bytes, &[5, 6]);
    queue.finish(true).unwrap();
    let chunk = queue.next(&[64, 0, 0, 64, 0, 0]).unwrap().unwrap();
    assert_eq!(chunk.bytes, &[7; 3]);
    queue.finish(true).unwrap();
    assert!(queue.next(&[0; CHANNELS as usize]).unwrap().is_none());
    let chunk = queue.next(&[0, 1, 0, 0, 0, 0]).unwrap().unwrap();
    assert_eq!(chunk.bytes, &[9]);
    queue.finish(true).unwrap();
    queue.clear(Channel::Display).unwrap();
    assert!(queue.next(&[64; CHANNELS as usize]).unwrap().is_none());
    assert!(queue.finish(true).is_err());
}
#[test]
fn t846_queue_bounds_charge_retained_allocations_and_message_overhead() {
    use blent_config::connection::queue::Scheduler;
    for capacity in [0, MAX_CREDIT + 1, u32::MAX] {
        assert!(Scheduler::new(capacity, 1, 1).is_err());
    }
    for messages in [0, 1025, usize::MAX] {
        assert!(Scheduler::new(64, messages, 1).is_err());
    }
    for quantum in [0, MAX_PAYLOAD + 1, u32::MAX] {
        assert!(Scheduler::new(64, 1, quantum).is_err());
    }
    let mut queue = Scheduler::new(8, 1, 4).unwrap();
    assert!(queue.offer(Channel::Control, vec![]).is_err());
    assert!(queue
        .offer(Channel::Control, vec![0; MAX_PAYLOAD as usize + 1])
        .is_err());
    let mut large = Vec::with_capacity(64);
    large.push(1);
    assert!(queue.offer(Channel::Control, large).is_err());
    queue.offer(Channel::Control, vec![8; 8]).unwrap();
    queue.next(&[8; CHANNELS as usize]).unwrap();
    queue.finish(true).unwrap();
    assert!(queue.offer(Channel::Control, vec![1]).is_err());
    queue.next(&[8; CHANNELS as usize]).unwrap();
    queue.finish(true).unwrap();
    queue.offer(Channel::Control, vec![1]).unwrap();
    assert!(queue.offer(Channel::Control, vec![1]).is_err());
}

#[test]
fn t846_seeded_duplex_flow_bounds_and_rejections() {
    use blent_config::connection::flow::Flow;
    let mut sender = Flow::new(1, 1, Channel::Display, 64).unwrap();
    let mut receiver = Flow::new(1, 1, Channel::Display, 64).unwrap();
    sender
        .receive(receiver.send(Kind::Credit, 0, 64).unwrap())
        .unwrap();
    let mut seed = 846u64;
    for _ in 0..1024 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let size = (seed % 64 + 1) as u32;
        let header = sender.send(Kind::Data, size, 0).unwrap();
        assert_eq!(sender.available(), 64 - size);
        let before = receiver.clone();
        let mut bad = header;
        bad.generation = 2;
        assert!(receiver.receive(bad).is_err());
        assert_eq!(receiver, before);
        receiver.receive(header).unwrap();
        assert!(receiver.send(Kind::Credit, 0, size).is_err());
        receiver.release(size).unwrap();
        sender
            .receive(receiver.send(Kind::Credit, 0, size).unwrap())
            .unwrap();
        assert_eq!(sender.available(), 64);
    }
    sender.send(Kind::Reset, 0, 0).unwrap();
    assert_eq!(sender.available(), 0);
}
#[test]
fn t846_seeded_queue_preserves_order_with_cancel_and_backpressure() {
    use blent_config::connection::queue::Scheduler;
    use std::collections::VecDeque;
    let mut queue = Scheduler::new(64, 8, 4).unwrap();
    let mut expected: [VecDeque<u8>; CHANNELS as usize] = Default::default();
    let mut seed = 846u64;
    for _ in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let index = (seed % CHANNELS as u64) as usize;
        let channel = Channel::try_from(index as u8 + 1).unwrap();
        let payload = vec![(seed >> 32) as u8; (seed % 80) as usize];
        if queue.offer(channel, payload.clone()).is_ok() {
            expected[index].extend(payload);
        }
        let credit = std::array::from_fn(|i| {
            if i == index {
                0
            } else {
                ((seed >> 32) % 8 + 1) as u32
            }
        });
        if let Some(slice) = queue.next(&credit).unwrap() {
            let index = slice.channel as usize - 1;
            assert!(slice.bytes.len() <= 4 && slice.bytes.len() <= credit[index] as usize);
            assert!(expected[index]
                .iter()
                .take(slice.bytes.len())
                .copied()
                .eq(slice.bytes.iter().copied()));
            let count = slice.bytes.len();
            let sent = seed % 3 != 0;
            if sent {
                expected[index].drain(..count);
            }
            queue.finish(sent).unwrap();
        }
    }
    while let Some(slice) = queue.next(&[64; CHANNELS as usize]).unwrap() {
        let index = slice.channel as usize - 1;
        let count = slice.bytes.len();
        assert!(expected[index]
            .iter()
            .take(count)
            .copied()
            .eq(slice.bytes.iter().copied()));
        expected[index].drain(..count);
        queue.finish(true).unwrap();
    }
    assert!(expected.iter().all(VecDeque::is_empty));
}

#[test]
fn t846_scheduler_rejects_oversized_credit_without_reserving() {
    use blent_config::connection::queue::Scheduler;
    for credit in [MAX_CREDIT + 1, u32::MAX] {
        let mut queue = Scheduler::new(64, 4, 4).unwrap();
        queue.offer(Channel::Control, vec![1; 8]).unwrap();
        // Iterator is empty only for rejection; retain this assertion across the
        // API's move from Option to an explicit Result error.
        assert_eq!(
            queue.next(&[credit; CHANNELS as usize]).into_iter().count(),
            0
        );
        assert!(queue.finish(true).is_err());
        assert_eq!(queue.next(&[64; CHANNELS as usize]).into_iter().count(), 1);
        queue.finish(true).unwrap();
    }
}
