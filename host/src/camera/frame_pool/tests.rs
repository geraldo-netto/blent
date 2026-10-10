use super::*;

#[tokio::test]
async fn t816_retained_readers_full_pool_and_generation_stay_immutable() {
    let mut pool = Pool::new(4, 2).unwrap();
    let first = pool.read(&mut &b"aaaa"[..]).await.unwrap().unwrap();
    let second = pool.read(&mut &b"bbbb"[..]).await.unwrap().unwrap();
    let pointer = first.as_ptr();
    assert!(pool.read(&mut &b"cccc"[..]).await.unwrap().is_none());
    assert_eq!(&**first, b"aaaa");
    assert_eq!(&**second, b"bbbb");
    drop(first);
    let third = pool.read(&mut &b"dddd"[..]).await.unwrap().unwrap();
    assert_eq!(third.as_ptr(), pointer);
    drop(pool); // Stop drops ownership, not outstanding frame leases.
    let mut replacement = Pool::new(4, 2).unwrap();
    let fresh = replacement.read(&mut &b"eeee"[..]).await.unwrap().unwrap();
    assert_eq!(&**second, b"bbbb");
    assert_eq!(&**third, b"dddd");
    assert_eq!(&**fresh, b"eeee");
}

#[tokio::test]
async fn t816_partial_eof_and_cancel_never_publish_incomplete_frames() {
    let mut pool = Pool::new(4, 2).unwrap();
    for len in 0..4 {
        assert!(pool.read(&mut &b"abcd"[..len]).await.is_err());
    }
    let held = pool.read(&mut &b"abcd"[..]).await.unwrap().unwrap();
    let (mut input, mut writer) = tokio::io::duplex(8);
    use tokio::io::AsyncWriteExt;
    writer.write_all(b"ef").await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), pool.read(&mut input))
            .await
            .is_err()
    );
    drop(pool); // cancelled connection discards its partial frame and pool
    assert_eq!(&**held, b"abcd");
    let mut next = Pool::new(4, 2).unwrap();
    writer.write_all(b"ghij").await.unwrap();
    assert_eq!(&**next.read(&mut input).await.unwrap().unwrap(), b"ghij");
}

#[test]
fn t816_invalid_capacities_and_lengths_are_bounded() {
    for bytes in [0, MAX_BYTES + 1, usize::MAX] {
        assert!(Pool::new(bytes, 2).is_err());
    }
    for slots in [0, 1, 17, usize::MAX] {
        assert!(Pool::new(1, slots).is_err());
    }
    for slots in 2..=16 {
        assert!(Pool::new(1, slots).is_ok());
    }
    assert!(Pool::new(MAX_BYTES, 2).is_ok());
    assert!(Pool::configured(4).is_ok());
}

#[tokio::test]
async fn t816_bounded_lease_schedule_property() {
    let mut seed = 17u32;
    for slots in 2..=4 {
        let mut pool = Pool::new(32, slots).unwrap();
        let mut held: Vec<(Arc<Vec<u8>>, u8)> = Vec::new();
        for step in 0..128u8 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            if !held.is_empty() && seed & 3 == 0 {
                held.remove(seed as usize % held.len());
            }
            let full = held.len() == slots;
            let source = [step; 32];
            let frame = pool.read(&mut source.as_slice()).await.unwrap();
            assert_eq!(frame.is_none(), full);
            if let Some(frame) = frame {
                held.push((frame, step));
            }
            for (frame, expected) in &held {
                assert!(frame.iter().all(|value| value == expected));
            }
        }
    }
}
