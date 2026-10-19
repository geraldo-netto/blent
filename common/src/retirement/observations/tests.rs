//! T1124: independent original obligations, bounded capacity and transition model.
use super::*;
use crate::retirement::{Admission, Pool};

#[test]
fn t1124_zero_capacity_and_original_identity_boundaries() {
    let mut empty = Observations::<0>::default();
    assert!(empty.complete());
    assert_eq!(
        empty.reserve().err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
    let pool = Pool::new(2).unwrap();
    let mut observations = Observations::<1>::default();
    let original = observations.reserve().unwrap();
    assert!(!observations.complete());
    assert!(observations.reserve().is_err());
    drop(original); // Failed creation admits replacement without a native proof.
    assert!(observations.complete());
    let first = pool.reserve().unwrap();
    observations.reserve().unwrap().bind(first.completion());
    assert!(!observations.complete());
    assert!(observations.reserve().is_err());
    drop(first);
    assert!(observations.complete());
    let next = observations.reserve().unwrap();
    let second = pool.reserve().unwrap();
    next.bind(second.completion());
    assert!(
        !observations.complete(),
        "old completion cannot retire new identity"
    );
    drop(second);
    assert!(observations.complete());
}

#[test]
fn t1124_bound_observation_survives_ticket_and_poisoned_mutex() {
    let pool = Pool::new(1).unwrap();
    let owner = pool.reserve().unwrap();
    let mut observations = Observations::<1>::default();
    let reservation = observations.reserve().unwrap();
    let original = reservation.0.clone();
    let panic = std::panic::catch_unwind(|| {
        let _held = original.lock().unwrap();
        panic!("T1124 injected lock-owner failure");
    });
    assert!(panic.is_err());
    reservation.bind(owner.completion());
    assert!(!observations.complete());
    assert!(observations.reserve().is_err());
    drop(owner);
    assert!(observations.complete());
    drop(observations.reserve().unwrap());
    assert!(observations.complete());
}

#[test]
fn t1124_two_slot_exhaustive_transition_model() {
    let pool = Pool::new(2).unwrap();
    // Exhaust every length-five word over eight creator/settlement operations:
    // 32,768 traces / 163,840 checked transitions. No fairness is assumed for
    // safety; eventual cleanup requires the explicit owner-drop operation.
    for word in 0..32768 {
        let mut model = Model::new(pool.clone());
        for shift in 0..5 {
            model.step((word >> (shift * 3)) & 7);
            model.check();
        }
        drop(model);
        assert_eq!(pool.active(), 0);
    }
}

struct Model {
    observations: Observations<2>,
    reservations: [Option<Reservation>; 2],
    owners: [Option<Admission>; 2],
    pool: Pool,
}
impl Model {
    fn new(pool: Pool) -> Self {
        Self {
            observations: Observations::default(),
            reservations: [None, None],
            owners: [None, None],
            pool,
        }
    }
    fn step(&mut self, operation: usize) {
        match operation {
            0..=1 => self.reserve(operation),
            2..=3 => self.bind(operation - 2),
            4..=5 => drop(self.reservations[operation - 4].take()),
            6..=7 => drop(self.owners[operation - 6].take()),
            _ => unreachable!(),
        }
    }
    fn obligations(&self) -> usize {
        self.reservations
            .iter()
            .filter(|value| value.is_some())
            .count()
            + self.owners.iter().filter(|value| value.is_some()).count()
    }
    fn reserve(&mut self, creator: usize) {
        if self.reservations[creator].is_some() {
            return;
        }
        let available = self.obligations() < 2;
        let result = self.observations.reserve();
        assert_eq!(result.is_ok(), available);
        self.reservations[creator] = result.ok();
    }
    fn bind(&mut self, creator: usize) {
        if self.owners[creator].is_some() {
            return;
        }
        let Some(original) = self.reservations[creator].take() else {
            return;
        };
        let native = self.pool.reserve().unwrap();
        original.bind(native.completion());
        self.owners[creator] = Some(native);
    }
    fn check(&self) {
        assert!(self.obligations() <= 2);
        assert_eq!(self.observations.complete(), self.obligations() == 0);
    }
}
