# T930: eliminate the discovery candidate copy

`Connections::poll` moves the candidate vector returned by discovery instead of
cloning its vector and peer strings before dropping the original. Discovery
errors still clear candidates and allow retained session cleanup to run before
returning the error. No discovery, admission, ticket or retirement contract changed.

The permanent normal-suite allocation regression was added before the fix and
failed for one peer: five allocations versus three for discovery alone.
After the fix, the same regression checks zero through four peers, candidate
identity/order, allocation counts, reallocations and requested bytes. Existing
stale-ticket, changed/replaced broker, ambiguous-peer and cleanup regressions
remain unchanged and pass. The bounded profile also checks immediate withdrawal
of original ticket authority on every changed snapshot.

Matched optimized local measurements used 100,000 unchanged polls and 256
prebuilt changed snapshots for each peer count. The same retained fake control
client feeds the production discovery and coordinator; no live USB connection,
broker, media startup or daemon scheduling is included. Thread-local allocation
counters and native thread CPU clocks are the existing test probes. Snapshot
construction is outside the measured first-change interval. These are warm,
synthetic single-run timings, not complete-application or battery measurements.
Raw logs, source hashes, Rust version and CPU/kernel identities are retained.

| Peers | Allocations/poll before → after | Requested bytes/poll before → after | CPU ns/poll before → after | First-change p95 ns before → after |
| --- | --- | --- | --- | --- |
| 1 | 5 → 3 | 648 → 536 | 97.8 → 67.0 | 130 → 100 |
| 2 | 7 → 4 | 824 → 600 | 141.6 → 110.0 | 181 → 150 |
| 3 | 9 → 5 | 1000 → 664 | 225.8 → 158.6 | 250 → 211 |
| 4 | 11 → 6 | 1176 → 728 | 309.0 → 210.1 | 351 → 260 |

Allocation savings are exact for this workload: 2–5 allocations and 112–448
requested bytes per poll. No reallocations occur. At the daemon's existing
100 ms polling interval, remaining snapshot projection costs only about
0.7–2.1 microseconds of measured thread CPU per second here. This does not
justify adding a snapshot cache and its invalidation state; current snapshots
continue to be projected and authority revalidated normally. Timing differences
are observations from one local run, not a repeatability or end-to-end claim.

Fresh locked/offline accessory-feature native collection passes 752 host library
tests; three pre-existing opt-in benchmark ignores are unchanged. Both new tests
run normally. All 32 production functions in discovery and coordination pass
individual 80% executable-line coverage, with none missing. Production/build
sources stayed immutable through collection and checking. Whole-project
cyclomatic checking passes 22,286 functions with none above 9. Formatting and
offline compilation pass. Android wake-driven polling remains separate T918 work.

```sh
cargo test --offline --locked -p blent --lib t930_idle_poll_retains_candidates -- --nocapture
cargo test --release --offline --locked -p blent --lib t930_profile_bounded -- --nocapture
/tmp/blent-t1007-venv/bin/python scripts/coverage/rust_collect.py --output /tmp/blent-t930-coverage --package blent --lib --feature accessory-transport
/tmp/blent-t1007-venv/bin/python scripts/coverage/report.py check --manifest /tmp/blent-t930-coverage/manifest.json --lcov /tmp/blent-t930-coverage/rust.lcov --native-sources /tmp/blent-t930-coverage/native-sources.json --rust-platform linux --scope host/src/connection_discovery.rs --scope host/src/display_connections.rs --output /tmp/blent-t930-coverage/per-function.json
cargo fmt --all -- --check
cargo check --offline --locked -p blent --features accessory-transport
/tmp/blent-t1007-venv/bin/python scripts/complexity/check.py
```
