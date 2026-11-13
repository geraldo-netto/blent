use super::*;
use blent_config::retirement::Pool;

#[test]
fn t1433_command_slots_are_bounded_and_cancelled_reservations_recover() {
    let commands = ObservedCommands::default();
    for _ in 0..64 {
        let receipts = commands.reserve().unwrap();
        assert!(!commands.complete());
        assert!(commands.reserve().is_err());
        drop(receipts);
        assert!(commands.complete());
    }
}

#[test]
fn t1433_child_and_collector_must_both_settle_in_either_order() {
    for first in [0, 1] {
        let pool = Pool::new(2).unwrap();
        let commands = ObservedCommands::default();
        let mut originals = [Some(pool.reserve().unwrap()), Some(pool.reserve().unwrap())];
        for (receipt, original) in commands.reserve().unwrap().into_iter().zip(&originals) {
            receipt.bind(original.as_ref().unwrap().completion());
        }
        assert!(!commands.complete());
        drop(originals[first].take());
        assert!(!commands.complete());
        assert!(commands.reserve().is_err());
        drop(originals[1 - first].take());
        assert!(commands.complete());
        assert!(commands.reserve().is_ok());
    }
}

#[tokio::test]
async fn t1433_unknown_original_prevents_successor_command_effects() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("executed");
    let pool = Pool::new(1).unwrap();
    let original = pool.reserve().unwrap();
    let commands = ObservedCommands::default();
    let [collector, unused] = commands.reserve().unwrap();
    collector.bind(original.completion());
    drop(unused);
    let args = [
        "-c",
        "printf started > \"$1\"",
        "fixture",
        marker.to_str().unwrap(),
    ];
    let next = commands.output("/bin/sh", &args);
    tokio::pin!(next);
    assert!(futures_util::poll!(&mut next).is_pending());
    assert!(!marker.exists());
    drop(original);
    assert!(next.await.unwrap().status.success());
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "started");
    while !commands.complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[tokio::test]
async fn t1433_failed_spawn_retires_collector_and_allows_later_commands() {
    let commands = ObservedCommands::default();
    for program in ["", "\0", "/nonexistent-blent-t1433/command"] {
        assert!(commands.output(program, &[]).await.is_err());
    }
    assert!(!commands
        .output("/bin/false", &[])
        .await
        .unwrap()
        .status
        .success());
    assert!(commands
        .output("/bin/true", &[])
        .await
        .unwrap()
        .status
        .success());
    while !commands.complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
