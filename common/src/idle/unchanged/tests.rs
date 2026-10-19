use super::*;

const LIVE: Conditions = Conditions {
    original_live: true,
    viewers_present: true,
    liveness_ready: true,
};

fn sample(sequence: u32, pts_us: i64) -> Sample {
    Sample {
        sequence,
        pts_us,
        ready_us: pts_us as u64 + 100_000,
        ack_us: pts_us as u64 + 110_000,
        keyframe: true,
    }
}

fn feed(policy: &mut Policy, previous: Sample, gap: i64, count: u32) -> Sample {
    let mut frame = previous;
    for _ in 0..count {
        frame = sample(frame.sequence + 1, frame.pts_us + gap);
        policy.observe(LIVE, frame);
    }
    frame
}

fn sparse_trial() -> (Policy, Sample) {
    let mut policy = Policy::new(true, true);
    let first = sample(0, 0);
    policy.observe(LIVE, first);
    let last = feed(&mut policy, first, 200_000, 16);
    assert_eq!(policy.phase(), Phase::SparseTrial);
    (policy, last)
}

fn unchanged_trial() -> (Policy, Sample) {
    let (mut policy, frame) = sparse_trial();
    let last = feed(&mut policy, frame, 500_000, 16);
    assert_eq!(policy.phase(), Phase::UnchangedTrial);
    (policy, last)
}

fn active() -> (Policy, Sample) {
    let (mut policy, frame) = unchanged_trial();
    let last = feed(&mut policy, frame, 1_000_000, 16);
    assert_eq!(policy.phase(), Phase::Active);
    (policy, last)
}

#[test]
fn t1109_requires_both_measured_windows_before_refresh_trial_then_sixteen_matches() {
    let mut baseline = Policy::new(true, true);
    assert_eq!(baseline.phase(), Phase::Baseline);
    assert_eq!(baseline.interval_ms(), 200);
    baseline.tick(LIVE, 0);
    let (trial, _) = sparse_trial();
    assert_eq!(trial.interval_ms(), 500);
    let (mut policy, mut frame) = unchanged_trial();
    assert_eq!(policy.interval_ms(), 1000);
    for _ in 0..15 {
        frame = feed(&mut policy, frame, 1_000_000, 1);
        assert_eq!(policy.phase(), Phase::UnchangedTrial);
    }
    feed(&mut policy, frame, 1_000_000, 1);
    assert_eq!(policy.phase(), Phase::Active);
}

#[test]
fn t1109_emit_oracle_covers_first_damage_cursor_refresh_and_unchanged_boundaries() {
    let policies = [
        Policy::new(true, true),
        sparse_trial().0,
        unchanged_trial().0,
        active().0,
    ];
    for (policy, expected) in policies.iter().zip([200, 500, 1000, 1000]) {
        for elapsed in 0..=1100 {
            assert_eq!(policy.emit(Change::Unchanged, elapsed), elapsed >= expected);
            for change in [
                Change::First,
                Change::Damage,
                Change::Cursor,
                Change::Refresh,
            ] {
                assert!(policy.emit(change, elapsed));
            }
        }
    }
}

#[test]
fn t1109_unqualified_recipe_or_decoder_never_admits_suppression() {
    for (recipe, decoder) in [(false, false), (false, true), (true, false)] {
        let mut policy = Policy::new(recipe, decoder);
        for sequence in 0..64 {
            policy.observe(LIVE, sample(sequence, i64::from(sequence) * 200_000));
        }
        policy.tick(LIVE, 20_000_000);
        assert_eq!(policy.phase(), Phase::Rejected);
        assert_eq!(policy.interval_ms(), 200);
    }
}

#[test]
fn t1109_original_authority_or_viewer_loss_is_terminal_in_every_phase() {
    for bits in 0..7 {
        let conditions = Conditions {
            original_live: bits & 1 != 0,
            viewers_present: bits & 2 != 0,
            liveness_ready: bits & 4 != 0,
        };
        for mut policy in [
            Policy::new(true, true),
            sparse_trial().0,
            unchanged_trial().0,
            active().0,
        ] {
            policy.tick(conditions, 60_000_000);
            assert_eq!(policy.phase(), Phase::Rejected);
            policy.observe(LIVE, sample(1000, 1_000_000_000));
            assert_eq!(policy.interval_ms(), 200);
        }
    }
    let (mut policy, frame) = active();
    policy.observe(
        Conditions::default(),
        sample(frame.sequence + 1, frame.pts_us + 1_000_000),
    );
    assert_eq!(policy.phase(), Phase::Rejected);
}

#[test]
fn t1109_lease_expiry_clock_reversal_and_future_ack_restore_compatibility() {
    for extra in [1_500_001, 2_000_000, u64::MAX / 2] {
        let (mut policy, frame) = active();
        policy.tick(LIVE, frame.ack_us + extra);
        assert_eq!(policy.phase(), Phase::Rejected);
    }
    let (mut policy, frame) = active();
    policy.tick(LIVE, frame.ack_us);
    policy.tick(LIVE, frame.ack_us - 1);
    assert_eq!(policy.phase(), Phase::Rejected);
    let (mut policy, frame) = active();
    policy.tick(LIVE, frame.ack_us - 1);
    assert_eq!(policy.interval_ms(), 200);
}

#[test]
fn t1109_delayed_ack_processing_and_exact_lease_boundary_remain_valid() {
    let (mut policy, frame) = active();
    policy.tick(LIVE, frame.ack_us + 500_000);
    let next = sample(frame.sequence + 1, frame.pts_us + 400_000);
    policy.observe(LIVE, next);
    policy.tick(LIVE, next.ack_us + 1_500_000);
    assert_eq!(policy.phase(), Phase::Active);
    assert_eq!(policy.interval_ms(), 1000);
}

#[test]
fn t1109_motion_resets_incomplete_trial_but_preserves_completed_qualification() {
    let (mut policy, frame) = unchanged_trial();
    let frame = feed(&mut policy, frame, 1_000_000, 8);
    let frame = feed(&mut policy, frame, 33_334, 4);
    let frame = feed(&mut policy, frame, 1_000_000, 15);
    assert_eq!(policy.phase(), Phase::UnchangedTrial);
    let frame = feed(&mut policy, frame, 1_000_000, 1);
    feed(&mut policy, frame, 33_334, 16);
    assert_eq!(policy.phase(), Phase::Active);
}

#[test]
fn t1109_bad_ack_cost_timestamp_sequence_and_key_progress_reject() {
    for fault in 0..5 {
        let (mut policy, frame) = active();
        let mut next = sample(frame.sequence + 1, frame.pts_us + 1_000_000);
        match fault {
            0 => next.sequence += 1,
            1 => next.pts_us = i64::MIN,
            2 => next.ack_us += 100_001,
            3 => {
                next.ready_us += 30_000;
                next.ack_us += 30_000;
            }
            _ => {
                next.keyframe = false;
                next.pts_us += 1_000_000;
                next.ready_us += 1_000_000;
                next.ack_us += 1_000_000;
            }
        }
        policy.observe(LIVE, next);
        assert_eq!(policy.phase(), Phase::Rejected);
    }
}

#[test]
fn t1109_refresh_trial_gap_boundaries_do_not_admit_outside_the_window() {
    for gap in [849_999, 850_000, 1_000_000, 1_250_000, 1_250_001] {
        let (mut policy, frame) = unchanged_trial();
        feed(&mut policy, frame, gap, 16);
        assert_eq!(
            policy.phase() == Phase::Active,
            (850_000..=1_250_000).contains(&gap)
        );
    }
}

fn initial(stage: u32) -> (Policy, Sample) {
    match stage {
        0 => (Policy::new(true, true), sample(0, 0)),
        1 => sparse_trial(),
        2 => unchanged_trial(),
        _ => active(),
    }
}

fn event(policy: &mut Policy, frame: Sample, selected: u32) -> Sample {
    let next = sample(frame.sequence + 1, frame.pts_us + 1_000_000);
    match selected {
        0 => {
            policy.observe(LIVE, next);
            return next;
        }
        1 => policy.tick(LIVE, frame.ack_us),
        2 => policy.tick(
            Conditions {
                original_live: false,
                ..LIVE
            },
            frame.ack_us,
        ),
        3 => policy.observe(
            Conditions {
                viewers_present: false,
                ..LIVE
            },
            next,
        ),
        4 => policy.tick(
            Conditions {
                liveness_ready: false,
                ..LIVE
            },
            frame.ack_us,
        ),
        _ => policy.reject(),
    }
    frame
}

#[test]
fn t1109_bounded_exhaustive_authority_cutoff_is_absorbing() {
    // Four original phases, six events, every trace of exactly four events.
    // Sequence/PTS values stay below 64/32,000,000; every allocation is local.
    for stage in 0..4 {
        for encoded in 0..6u32.pow(4) {
            let (mut policy, mut frame) = initial(stage);
            let mut digits = encoded;
            let mut rejected = false;
            for _ in 0..4 {
                let selected = digits % 6;
                digits /= 6;
                rejected = rejected || selected >= 2;
                frame = event(&mut policy, frame, selected);
                assert_eq!(
                    policy.phase() == Phase::Rejected,
                    rejected,
                    "T1109 stage={stage}, trace={encoded}, event={selected}"
                );
            }
        }
        eprintln!("T1109 completed phase {stage}: 1296 traces / 5184 transitions");
    }
}
