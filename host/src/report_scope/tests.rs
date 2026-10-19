//! T1085: metadata authority cutoff, including retained aliases and unwind.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn t1085_scope_never_revives_after_revoke_replacement_or_owner_drop() {
    let owner = Owner::default();
    assert!(owner.permit().is_err());
    owner.revoke();
    owner.begin();
    let old = owner.permit().unwrap();
    let calls = AtomicUsize::new(0);
    old.publish(|| {
        calls.fetch_add(1, Ordering::SeqCst);
    });
    owner.revoke();
    assert!(owner.permit().is_err());
    old.publish(|| panic!("T1085 revoked callback ran"));
    owner.begin();
    let next = owner.permit().unwrap();
    old.publish(|| panic!("T1085 old callback revived"));
    next.publish(|| {
        calls.fetch_add(1, Ordering::SeqCst);
    });
    drop(owner);
    next.publish(|| panic!("T1085 callback survived owner cutoff"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn t1085_panicking_publication_poison_cannot_bypass_cutoff() {
    let owner = Owner::default();
    owner.begin();
    let old = owner.permit().unwrap();
    assert!(
        std::panic::catch_unwind(|| old.publish(|| panic!("T1085 publication panic"))).is_err()
    );
    owner.begin();
    old.publish(|| panic!("T1085 poisoned scope survived replacement"));
    owner.permit().unwrap().publish(|| {});
    owner.revoke();
}

#[test]
fn t1085_cutoff_waits_for_exact_original_publication_before_settling() {
    let owner = Arc::new(Owner::default());
    owner.begin();
    let permit = owner.permit().unwrap();
    let (entered, entry) = std::sync::mpsc::channel();
    let (release, gate) = std::sync::mpsc::channel();
    let publishing = permit.clone();
    let worker = std::thread::spawn(move || {
        publishing.publish(|| {
            entered.send(()).unwrap();
            let _ = gate.recv();
        })
    });
    entry.recv().unwrap();
    let cutoff = owner.clone();
    let (sent, done) = std::sync::mpsc::channel();
    let revoker = std::thread::spawn(move || {
        cutoff.revoke();
        sent.send(()).unwrap();
    });
    let withheld = done
        .recv_timeout(std::time::Duration::from_millis(100))
        .is_err();
    release.send(()).unwrap();
    worker.join().unwrap();
    revoker.join().unwrap();
    permit.publish(|| panic!("T1085 settled cutoff still publishes"));
    assert!(withheld, "T1085 in-flight publication was not fenced");
}

// Independent bounded oracle: at most one saved alias may publish, and only
// after the latest Begin. Revoke/owner loss admit no writes from any alias.
struct Model {
    owner: Option<Owner>,
    aliases: Vec<Permit>,
    current: Option<usize>,
    observed: std::cell::Cell<Option<usize>>,
    expected: Option<usize>,
}
impl Model {
    fn new() -> Self {
        Self {
            owner: None,
            aliases: Vec::with_capacity(6),
            current: None,
            observed: std::cell::Cell::new(None),
            expected: None,
        }
    }
    fn begin(&mut self) {
        let owner = self.owner.get_or_insert_with(Owner::default);
        owner.begin();
        assert!(self.aliases.len() < 6);
        self.current = Some(self.aliases.len());
        self.aliases.push(owner.permit().unwrap());
    }
    fn revoke(&mut self) {
        if let Some(owner) = &self.owner {
            owner.revoke();
        }
        self.current = None;
    }
    fn publish(&mut self) {
        for (index, permit) in self.aliases.iter().enumerate() {
            permit.publish(|| self.observed.set(Some(index)));
            if self.current == Some(index) {
                self.expected = Some(index);
            }
            assert_eq!(
                self.observed.get(),
                self.expected,
                "T1085 forbidden old alias write"
            );
        }
    }
    fn step(&mut self, action: usize) {
        match action {
            0 => self.begin(),
            1 => self.revoke(),
            2 => self.publish(),
            3 => {
                self.owner = None;
                self.current = None;
            }
            _ => unreachable!("T1085 finite four-action input domain"),
        }
    }
}
#[test]
fn t1085_exhaustive_six_step_publication_histories_preserve_original_cutoff() {
    for mut trace in 0..4usize.pow(6) {
        let mut model = Model::new();
        for _ in 0..6 {
            model.step(trace % 4);
            trace /= 4;
        }
        model.owner = None;
        model.current = None;
        model.publish();
    }
}
