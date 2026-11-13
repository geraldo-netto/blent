use super::*;
use std::collections::HashSet;

fn fixed_entropy(bytes: &mut [u8]) -> anyhow::Result<()> {
    assert_eq!(bytes.len(), 16);
    bytes.fill(0xa5);
    Ok(())
}
fn denied_entropy(_: &mut [u8]) -> anyhow::Result<()> {
    anyhow::bail!("T1445 entropy unavailable")
}
#[test]
fn t1445_entropy_failure_issues_nothing_and_success_initializes_once() {
    let storage = Mutex::new(None);
    assert!(issue_using(&storage, denied_entropy).is_err());
    assert!(storage.lock().unwrap().is_none());
    let first = issue_using(&storage, fixed_entropy).unwrap();
    let next = issue_using(&storage, denied_entropy).unwrap();
    assert_eq!(first.namespace, next.namespace);
    assert_eq!((first.serial, next.serial), (0, 1));
}
#[test]
fn t1445_counter_exhaustion_and_poison_never_reuse_an_issued_stamp() {
    let mut sequence = Sequence {
        namespace: 1,
        next: u64::MAX - 1,
    };
    assert_eq!(sequence.issue().unwrap().serial, u64::MAX - 1);
    assert!(sequence.issue().is_err());
    assert!(sequence.issue().is_err());
    let storage = Mutex::new(None);
    std::thread::scope(|scope| {
        assert!(scope
            .spawn(|| {
                let _owner = storage.lock().unwrap();
                panic!("T1445 poisoned identity owner");
            })
            .join()
            .is_err());
    });
    assert!(issue_using(&storage, fixed_entropy).is_err());
}
#[test]
fn t1445_kwin_names_do_not_consume_entropy_or_sequence() {
    let base = DeviceIdentity::for_instance(4095).unwrap();
    for session in ["", "wayland", "unknown"] {
        let result =
            identity_using(&base, session, || panic!("KWin must not issue a stamp")).unwrap();
        assert_eq!(
            (result.touch, result.pen, result.pointer),
            (base.touch.clone(), base.pen.clone(), base.pointer.clone())
        );
    }
    assert!(identity_using(&base, "x11", || Err(io::Error::other("exhausted"))).is_err());
}
fn assert_products(identity: &DeviceIdentity, base: &DeviceIdentity) {
    assert_eq!(identity.product_touch, base.product_touch);
    assert_eq!(identity.product_pen, base.product_pen);
    assert_eq!(identity.product_pointer, base.product_pointer);
}
#[test]
fn t1445_bounded_instance_namespace_sequence_names_preserve_products() {
    let mut names = HashSet::new();
    for instance in 0..=4095 {
        let base = DeviceIdentity::for_instance(instance).unwrap();
        for (namespace, serial) in [
            (0, 0),
            (0, u64::MAX - 1),
            (u128::MAX, 0),
            (u128::MAX, u64::MAX - 1),
        ] {
            let created = identity_using(&base, "x11", || Ok(Stamp { namespace, serial })).unwrap();
            assert_products(&created, &base);
            for name in [created.touch, created.pen, created.pointer] {
                assert!(name.len() < 80, "kernel uinput name bound");
                assert!(name.is_ascii());
                assert!(names.insert(name), "creation identity collision");
            }
        }
    }
    assert_eq!(names.len(), 4096 * 4 * 3);
}
#[test]
fn t1445_shared_issuer_serializes_concurrent_creations() {
    let storage = Mutex::new(None);
    let issued = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    (0..32)
                        .map(|_| issue_using(&storage, fixed_entropy).unwrap().serial)
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect::<HashSet<_>>()
    });
    assert_eq!(issued.len(), 256);
    assert_eq!(issued.iter().max(), Some(&255));
}
