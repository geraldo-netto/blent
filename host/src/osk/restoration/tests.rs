use super::*;
use crate::process_retirement::Registry;

fn admitted(registry: &Registry, pool: &Pool) -> Restoration {
    Restoration {
        admission: Some(registry.reserve(pool).unwrap()),
        original: None,
    }
}

async fn complete(proof: &Completion) {
    while !proof.is_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[test]
fn t1441_cached_pool_failure_preserves_error_and_success_keeps_one_slot() {
    let failed = OnceLock::new();
    failed
        .set(Err(io::Error::other("original failure")))
        .unwrap();
    for _ in 0..2 {
        let error = pool(&failed).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(error.to_string(), "original failure");
    }
    let storage = OnceLock::new();
    let first = pool(&storage).unwrap().reserve().unwrap();
    assert!(pool(&storage).unwrap().reserve().is_err());
    drop(first);
    assert!(pool(&storage).unwrap().reserve().is_ok());
}

#[tokio::test]
async fn t1441_unused_reservation_releases_and_cutoff_rejects_successors() {
    let registry = Registry::default();
    let pool = Pool::new(1).unwrap();
    let owner = admitted(&registry, &pool);
    let proof = owner.admission.as_ref().unwrap().completion();
    assert!(registry.reserve(&pool).is_err());
    drop(owner);
    assert!(proof.is_complete());
    let replacement = admitted(&registry, &pool);
    registry.close();
    assert!(registry.reserve(&pool).is_err());
    drop(replacement);
    registry.drain().await;
    assert_eq!(pool.active(), 0);
}

#[tokio::test]
async fn t1441_cancelled_gate_wait_has_no_original_effect_or_retained_slot() {
    let registry = Registry::default();
    let pool = Pool::new(1).unwrap();
    let gate = Gate::default();
    let predecessor = gate.acquire().await.unwrap();
    let prior_proof = predecessor.completion();
    let owner = admitted(&registry, &pool);
    let proof = owner.admission.as_ref().unwrap().completion();
    let mut waiting = Box::pin(owner.restore(&gate));
    assert!(futures_util::poll!(waiting.as_mut()).is_pending());
    drop(waiting);
    assert!(proof.is_complete());
    assert!(!prior_proof.is_complete());
    assert!(gate.commands.complete());
    drop(predecessor);
    complete(&prior_proof).await;
    registry.drain().await;
}

#[tokio::test]
async fn t1441_cancelled_owner_keeps_exact_original_through_cutoff() {
    let registry = Registry::default();
    let pool = Pool::new(1).unwrap();
    let source = Pool::new(1).unwrap();
    let original = source.reserve().unwrap();
    let mut owner = admitted(&registry, &pool);
    owner.original = Some(original.completion());
    let proof = owner.admission.as_ref().unwrap().completion();
    drop(owner);
    registry.close();
    let mut draining = Box::pin(registry.drain());
    assert!(futures_util::poll!(draining.as_mut()).is_pending());
    assert!(!proof.is_complete());
    assert!(pool.reserve().is_err());
    drop(original);
    draining.await;
    assert!(proof.is_complete());
    assert_eq!(pool.active(), 0);
}

#[tokio::test]
async fn t1441_operation_exhaustion_releases_unused_restoration() {
    let registry = Registry::default();
    let pool = Pool::new(1).unwrap();
    let gate = Gate::default();
    let first = gate.pool().unwrap().reserve().unwrap();
    let second = gate.pool().unwrap().reserve().unwrap();
    let owner = admitted(&registry, &pool);
    let proof = owner.admission.as_ref().unwrap().completion();
    owner.restore(&gate).await;
    assert!(proof.is_complete());
    assert!(gate.commands.complete());
    drop((first, second));
    let next = gate.acquire().await.unwrap();
    let next_proof = next.completion();
    drop(next);
    complete(&next_proof).await;
    registry.drain().await;
}
