//! T930: bounded, synthetic production discovery/coordinator profiling.
//! Compiled only into host unit tests; no USB device or live broker is touched.
use super::*;
use crate::allocation_probe::{cpu_ns, measure, Counts};
use std::hint::black_box;

const IDLE_POLLS: u64 = 100_000;
const CHANGES: u64 = 256;

fn fixture(count: u32) -> Fixture {
    let mut f = Fixture::new(count);
    let links: Vec<_> = (1..=u64::from(count)).map(|id| (id, id + 1, 1)).collect();
    f.links(&links);
    f
}

fn idle(f: &mut Fixture, discovery_only: bool) -> serde_json::Value {
    let cpu = cpu_ns();
    let wall = Instant::now();
    let (_, allocations) = measure(|| {
        for _ in 0..IDLE_POLLS {
            if discovery_only {
                black_box(f.connections.source.poll(f.now).unwrap());
            } else {
                f.poll();
                black_box(&f.connections.candidates);
            }
        }
    });
    serde_json::json!({
        "polls": IDLE_POLLS, "allocations": allocations,
        "cpu_ns": cpu_ns() - cpu, "wall_ns": wall.elapsed().as_nanos()
    })
}

fn changed_view(
    f: &mut Fixture,
    count: u32,
    epoch: u64,
) -> blent_config::connection::control_view::View {
    f.revision += 1;
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: f.revision,
            local: peer(1),
        })
        .unwrap();
    for id in 1..=u64::from(count) {
        assembler
            .push(Notice::Link {
                revision: f.revision,
                connection: id,
                ticket: Some(PairingTicket {
                    peer: peer(id + 1),
                    epoch,
                }),
                state: State::Authenticated,
                trusted: false,
            })
            .unwrap();
    }
    f.model
        .publish(
            assembler
                .push(Notice::End {
                    revision: f.revision,
                })
                .unwrap()
                .unwrap(),
        )
        .unwrap()
}

fn add(total: &mut Counts, counts: Counts) {
    total.allocations += counts.allocations;
    total.reallocations += counts.reallocations;
    total.requested_bytes += counts.requested_bytes;
}

fn first_change(f: &mut Fixture, count: u32) -> serde_json::Value {
    let views: Vec<_> = (2..=CHANGES + 1)
        .map(|epoch| changed_view(f, count, epoch))
        .collect();
    let original = f.connections.candidates.clone();
    let mut elapsed = Vec::with_capacity(CHANGES as usize);
    let mut allocations = Counts::default();
    let mut cpu = 0;
    for view in views {
        f.source.lock().unwrap().view = Some(view);
        let clock = cpu_ns();
        let start = Instant::now();
        let (_, counts) = measure(|| f.poll());
        elapsed.push(start.elapsed().as_nanos());
        cpu += cpu_ns() - clock;
        add(&mut allocations, counts);
        assert_eq!(f.connections.candidates.len(), count as usize);
        for candidate in &original {
            assert!(!f.connections.source.current(candidate));
        }
    }
    elapsed.sort_unstable();
    serde_json::json!({
        "changes": CHANGES, "allocations": allocations, "cpu_ns": cpu,
        "p50_wall_ns": elapsed[elapsed.len()/2],
        "p95_wall_ns": elapsed[elapsed.len()*95/100],
        "max_wall_ns": elapsed.last().unwrap()
    })
}

#[test]
fn t930_profile_bounded_idle_and_first_change() {
    let rows: Vec<_> = (1..=4)
        .map(|count| {
            let mut f = fixture(count);
            serde_json::json!({
                "peers": count, "discovery": idle(&mut f, true),
                "coordinator": idle(&mut f, false), "first_change": first_change(&mut f, count)
            })
        })
        .collect();
    println!("T930_PROFILE {}", serde_json::json!({"rows": rows}));
}
