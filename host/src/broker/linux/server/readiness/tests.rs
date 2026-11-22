use super::*;
fn ready(id: u64, polls: &mut usize, _: &mut Context<'_>) -> Poll<u64> {
    *polls += 1;
    Poll::Ready(id)
}
fn pending(_: u64, polls: &mut usize, _: &mut Context<'_>) -> Poll<u64> {
    *polls += 1;
    Poll::Pending
}
#[test]
fn t1778_cursor_rotates_and_new_ids_cannot_extend_an_old_round() {
    let mut cursor = Cursor::default();
    let mut records = BTreeMap::from([(1, 0), (2, 0), (3, 0)]);
    let mut cx = Context::from_waker(futures_util::task::noop_waker_ref());
    assert_eq!(cursor.poll(&mut records, ready, &mut cx), Poll::Ready(1));
    // Independent FIFO oracle: snapshot the current membership only when the
    // preceding round drains. Production uses no queue or copied membership.
    let mut round = std::collections::VecDeque::from([2, 3]);
    for id in 4..=35 {
        records.insert(id, 0);
        if round.is_empty() {
            round.extend(records.keys().copied());
        }
        assert_eq!(
            cursor.poll(&mut records, ready, &mut cx),
            Poll::Ready(round.pop_front().unwrap())
        );
    }
}
#[test]
fn t1778_pending_turn_polls_each_original_owner_at_most_once() {
    let mut cursor = Cursor::default();
    let mut records = BTreeMap::from([(1, 0), (2, 0), (3, 0)]);
    let mut cx = Context::from_waker(futures_util::task::noop_waker_ref());
    assert_eq!(cursor.poll(&mut records, ready, &mut cx), Poll::Ready(1));
    records.values_mut().for_each(|count| *count = 0);
    records.insert(4, 0);
    assert!(cursor.poll(&mut records, pending, &mut cx).is_pending());
    assert!(records.values().all(|count| *count == 1));
    assert!(cursor.poll(&mut records, pending, &mut cx).is_pending());
    assert!(records.values().all(|count| *count == 2));
}
#[test]
fn t1778_removed_cursor_id_and_empty_round_do_not_stall_replacement() {
    let mut cursor = Cursor::default();
    let mut records = BTreeMap::from([(1, 0), (2, 0)]);
    let mut cx = Context::from_waker(futures_util::task::noop_waker_ref());
    assert_eq!(cursor.poll(&mut records, ready, &mut cx), Poll::Ready(1));
    records.clear();
    assert!(cursor.poll(&mut records, ready, &mut cx).is_pending());
    records.insert(u64::MAX, 0);
    assert_eq!(
        cursor.poll(&mut records, ready, &mut cx),
        Poll::Ready(u64::MAX)
    );
    assert_eq!(
        cursor.poll(&mut records, ready, &mut cx),
        Poll::Ready(u64::MAX)
    );
}
