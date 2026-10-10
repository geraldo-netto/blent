use blent_config::connection::{
    flow::Flow,
    mux::Multiplexer,
    queue::Scheduler,
    wire::{Channel, Header, Kind, CHANNELS, MAX_CREDIT},
};
fn mux() -> Multiplexer {
    let mut owner = Multiplexer::new(7, 9, 64, 16, 4).unwrap();
    for id in 1..CHANNELS { owner.activate(Channel::try_from(id).unwrap(), 9).unwrap(); }
    owner
}
fn frame(owner: &mut Multiplexer) -> Option<(Header, Vec<u8>)> {
    let frame = owner.next().unwrap()?;
    let value = (frame.header, frame.bytes.to_vec());
    owner.finish(true).unwrap();
    Some(value)
}
fn grant(left: &mut Multiplexer, right: &mut Multiplexer) {
    for channel in 1..=CHANNELS {
        let (header, bytes) = frame(left).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(header.channel as u8, channel);
        assert_eq!(header.kind, Kind::Credit);
        assert!(!right.receive(header).unwrap());
        let (header, _) = frame(right).unwrap();
        assert!(!left.receive(header).unwrap());
    }
    assert!(frame(left).is_none());
    assert!(frame(right).is_none());
}
#[test]
fn t846_mux_metadata_bootstraps_and_coalesces_consumer_credit() {
    let (mut left, mut right) = (mux(), mux());
    grant(&mut left, &mut right);
    let bytes = vec![42; 8];
    let address = bytes.as_ptr();
    left.offer(Channel::Display, bytes).unwrap();
    for offset in [0, 4] {
        let reserved = left.next().unwrap().unwrap();
        assert_eq!(reserved.bytes.as_ptr(), address.wrapping_add(offset));
        let header = reserved.header;
        assert!(left.next().unwrap().is_none());
        assert!(right.receive(header).unwrap());
        left.finish(true).unwrap();
    }
    assert!(frame(&mut right).is_none());
    right.release(Channel::Display, 3).unwrap();
    right.release(Channel::Display, 5).unwrap();
    let (header, _) = frame(&mut right).unwrap();
    assert_eq!(header.credit, 8);
    assert_eq!(header.sequence, 1);
    assert!(!left.receive(header).unwrap());
    assert!(frame(&mut right).is_none());
    assert!(right.release(Channel::Display, 1).is_err());
}
#[test]
fn t846_mux_terminal_drains_inflight_without_stalling_other_channels() {
    let (mut left, mut right) = (mux(), mux());
    grant(&mut left, &mut right);
    left.offer(Channel::Display, vec![1; 8]).unwrap();
    left.offer(Channel::Control, vec![2; 8]).unwrap();
    let (first, _) = frame(&mut left).unwrap();
    assert_eq!(first.channel, Channel::Control);
    assert!(right.receive(first).unwrap());
    right.release(first.channel, first.length).unwrap();
    let pending = left.next().unwrap().unwrap().header;
    assert_eq!(pending.channel, Channel::Display);
    right.close(Channel::Display, Kind::Reset).unwrap();
    assert!(!right.receive(pending).unwrap()); // already authorized, discarded after local stop
    assert!(right.offer(Channel::Display, vec![4]).is_err());
    assert!(right.release(Channel::Display, 4).is_err());
    let (credit, _) = frame(&mut right).unwrap();
    assert_eq!(credit.kind, Kind::Credit);
    left.receive(credit).unwrap();
    let (reset, _) = frame(&mut right).unwrap();
    assert_eq!(reset.kind, Kind::Reset);
    left.receive(reset).unwrap();
    assert!(!left.retired(Channel::Display));
    left.finish(true).unwrap();
    let (ack, _) = frame(&mut left).unwrap();
    assert_eq!(ack.kind, Kind::Reset);
    assert_eq!(ack.sequence, 2);
    assert!(!right.receive(ack).unwrap());
    assert!(!left.retired(Channel::Display));
    assert!(left.replace_channel(Channel::Display, 10).is_err());
    left.consumer_retired(Channel::Display, 9).unwrap();
    right.consumer_retired(Channel::Display, 9).unwrap();
    assert!(left.retired(Channel::Display));
    assert!(right.retired(Channel::Display));
    assert!(right.receive(pending).is_err());
    assert!(right.close(Channel::Display, Kind::Close).is_err());
    let (control, bytes) = frame(&mut left).unwrap();
    assert_eq!(control.channel, Channel::Control);
    assert_eq!(bytes, vec![2; 4]);
    assert!(right.receive(control).unwrap());
}
#[test]
fn t846_mux_bounded_fairness_and_stalled_consumer() {
    let (mut left, mut right) = (mux(), mux());
    grant(&mut left, &mut right);
    // Display never releases its receive bytes; control/audio/camera keep progressing.
    let mut sent = [0; CHANNELS as usize];
    let mut rounds = [0; CHANNELS as usize];
    for round in 0..4096 {
        for id in 1..=CHANNELS {
            let channel = Channel::try_from(id).unwrap();
            let _ = left.offer(channel, vec![id; 4]);
        }
        let (header, bytes) = frame(&mut left).unwrap();
        assert_eq!(header.kind, Kind::Data);
        assert!(right.receive(header).unwrap());
        let i = header.channel as usize - 1;
        sent[i] += bytes.len();
        if i != 1 {
            assert!(round - rounds[i] <= CHANNELS as usize);
            rounds[i] = round;
            right.release(header.channel, header.length).unwrap();
            let (credit, _) = frame(&mut right).unwrap();
            left.receive(credit).unwrap();
        }
    }
    assert_eq!(sent[1], 64);
    for id in [0, 2, 3, 4, 5] {
        assert!(sent[id] > 3000);
    }
}
#[test]
fn t846_mux_invalid_input_and_failed_committed_writes_retire_connection() {
    for capacity in [0, MAX_CREDIT + 1, u32::MAX] {
        assert!(Multiplexer::new(1, 1, capacity, 1, 1).is_err());
    }
    assert!(Multiplexer::new(0, 1, 1, 1, 1).is_err());
    assert!(Multiplexer::new(1, 0, 1, 1, 1).is_err());
    assert!(Multiplexer::new(1, 1, 1, 0, 1).is_err());
    assert!(Multiplexer::new(1, 1, 1, 1, 0).is_err());
    let mut left = mux();
    assert!(left.finish(true).is_err());
    assert!(left.close(Channel::Control, Kind::Data).is_err());
    let header = left.next().unwrap().unwrap().header;
    assert_eq!(header.sequence, 0);
    left.finish(false).unwrap();
    assert!(left.next().is_err());
    assert!(left.offer(Channel::Control, vec![1]).is_err());
    assert!(left.receive(header).is_err());
    assert!(left.release(Channel::Control, 1).is_err());
    assert!(left.close(Channel::Control, Kind::Reset).is_err());
    assert!(!left.retired(Channel::Control));
    let (mut left, mut right) = (mux(), mux());
    grant(&mut left, &mut right);
    left.offer(Channel::Control, vec![1]).unwrap();
    left.next().unwrap().unwrap();
    left.finish(false).unwrap();
    assert!(left.next().is_err());
}
#[test]
fn t846_flow_drain_validates_bounds_order_and_terminal_acknowledgement() {
    let mut left = Flow::new(7, 9, Channel::Control, 64).unwrap();
    let mut right = left.clone();
    right
        .receive(left.send(Kind::Credit, 0, 64).unwrap())
        .unwrap();
    let body = right.send(Kind::Data, 16, 0).unwrap();
    assert!(left.drain(body).is_err());
    let close = left.terminal(Kind::Close).unwrap();
    assert!(!left.retired());
    assert!(left.terminal(Kind::Reset).is_err());
    for invalid in [
        Header { length: 65, ..body },
        Header { session: 8, ..body },
        Header {
            sequence: 1,
            ..body
        },
    ] {
        let before = left.clone();
        assert!(left.drain(invalid).is_err());
        assert_eq!(left, before);
    }
    left.drain(body).unwrap();
    assert!(left.drain(body).is_err());
    right.receive(close).unwrap();
    let ack = right.terminal(Kind::Close).unwrap();
    assert!(right.retired());
    left.drain(ack).unwrap();
    assert!(left.retired());
    assert!(left.drain(ack).is_err());
    let queue = Scheduler::new(1, 1, 1).unwrap();
    assert!(queue.reserved().is_none());
}

#[test]
fn t846_channel_replacement_waits_for_peer_writer_and_consumer_retirement() {
    let (mut left, mut right) = (mux(), mux());
    grant(&mut left, &mut right);
    assert!(left.consumer_retired(Channel::Camera, 9).is_err());
    assert!(left.replace_channel(Channel::Camera, 10).is_err());
    left.close(Channel::Camera, Kind::Close).unwrap();
    assert!(left.consumer_retired(Channel::Camera, 8).is_err());
    left.consumer_retired(Channel::Camera, 9).unwrap();
    assert!(!left.retired(Channel::Camera));
    let (close, _) = frame(&mut left).unwrap();
    right.receive(close).unwrap();
    let ack = right.next().unwrap().unwrap().header;
    right.consumer_retired(Channel::Camera, 9).unwrap();
    assert!(!right.retired(Channel::Camera));
    left.receive(ack).unwrap();
    assert!(left.retired(Channel::Camera));
    right.finish(true).unwrap();
    assert!(right.retired(Channel::Camera));
    for generation in [0, 9, u64::MAX] {
        assert!(left.replace_channel(Channel::Camera, generation).is_err());
    }
    left.replace_channel(Channel::Camera, 10).unwrap();
    right.replace_channel(Channel::Camera, 10).unwrap();
    assert!(!left.retired(Channel::Camera));
    assert!(left.consumer_retired(Channel::Camera, 9).is_err());
    assert!(left.receive(ack).is_err());
    assert!(frame(&mut left).is_none(), "T849: replacement remains inactive");
    assert!(frame(&mut right).is_none());
    left.activate(Channel::Camera, 10).unwrap();
    right.activate(Channel::Camera, 10).unwrap();
    let (credit, _) = frame(&mut left).unwrap();
    assert_eq!(credit.generation, 10);
    assert_eq!(credit.sequence, 0);
    right.receive(credit).unwrap();
    let (credit, _) = frame(&mut right).unwrap();
    left.receive(credit).unwrap();
    left.offer(Channel::Camera, vec![7; 4]).unwrap();
    let (header, bytes) = frame(&mut left).unwrap();
    assert_eq!(header.generation, 10);
    assert_eq!(bytes, vec![7; 4]);
    assert!(right.receive(header).unwrap());
    right.release(Channel::Camera, 4).unwrap();
}

#[test]
fn t849_unleased_channels_never_grant_initial_credit() {
    let mut owner = Multiplexer::new(7, 9, 64, 16, 4).unwrap();
    assert_eq!(frame(&mut owner).unwrap().0.channel, Channel::Management);
    assert!(frame(&mut owner).is_none());
    assert!(owner.offer(Channel::Display, vec![1]).is_err());
}

#[test]
fn t849_activation_rejects_stale_duplicate_closed_and_failed_without_mutation() {
    let mut owner = Multiplexer::new(7, 9, 64, 16, 4).unwrap();
    let management = frame(&mut owner).unwrap().0;
    assert_eq!(management.channel, Channel::Management);
    for id in 1..CHANNELS {
        let channel = Channel::try_from(id).unwrap();
        for generation in [0, 8, 10, i64::MAX as u64, u64::MAX] {
            assert!(owner.activate(channel, generation).is_err());
        }
        let credit = Header { channel, ..management };
        assert!(owner.receive(credit).is_err());
        assert!(owner.receive(Header { kind: Kind::Data, length: 1, credit: 0, ..credit }).is_err());
        assert!(owner.release(channel, 1).is_err());
        assert!(owner.offer(channel, vec![1]).is_err());
        assert!(frame(&mut owner).is_none());
        owner.activate(channel, 9).unwrap();
        assert!(owner.activate(channel, 9).is_err());
        let emitted = frame(&mut owner).unwrap().0;
        assert_eq!((emitted.channel, emitted.sequence, emitted.credit), (channel, 0, 64));
        assert!(frame(&mut owner).is_none());
        owner.receive(credit).unwrap(); // rejected headers never advanced sequence
        owner.offer(channel, vec![1]).unwrap();
        assert_eq!(frame(&mut owner).unwrap().0.sequence, 1);
    }
    assert!(owner.activate(Channel::Management, 9).is_err());
    owner.close(Channel::Camera, Kind::Reset).unwrap();
    assert!(owner.activate(Channel::Camera, 9).is_err());
    frame(&mut owner).unwrap();
    owner.receive(management).unwrap();
    owner.offer(Channel::Management, vec![1]).unwrap();
    assert_eq!(frame(&mut owner).unwrap().0.channel, Channel::Management);
    let mut inactive = Multiplexer::new(7, 9, 64, 16, 4).unwrap();
    inactive.receive(Header { channel: Channel::Control, kind: Kind::Close, credit: 0, ..management }).unwrap();
    assert!(inactive.activate(Channel::Control, 9).is_err());
    inactive.next().unwrap().unwrap();
    inactive.finish(false).unwrap();
    assert!(inactive.activate(Channel::Display, 9).is_err());
}
