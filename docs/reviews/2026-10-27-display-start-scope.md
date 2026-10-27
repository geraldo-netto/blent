# Preserve targeted display Start scope (T1275)

T1274 exposed an existing automatic USB priority rule through manual Start. At
full capacity, reenabling a USB peer could select another tablet's ADB ticket
for replacement. A separate eligibility calculation could also displace an ADB
owner while selected USB intent was disabled or waiting.

Slots now owns one bounded collection of explicit policy exceptions. Suppressed
targets remain disabled; manually enabled USB targets wait for free capacity.
Untouched automatic USB discovery retains its established priority behavior.
ADB eligibility derives from the same authoritative plan. Neither a waiting
manual target nor stale disabled selection reserves a sibling's slot.

The collection admits at most 1,024 exceptions, including cooperative USB intent.
Enabling ADB removes its suppression exception because ADB admission already
preserves existing owners. Each key has one authoritative policy, validated
before mutation. Exhaustion rejects atomically; clearing or changing policy never
retires a display ticket. Policy remains owned until daemon exit or explicit
removal, independent of connection loss and native cleanup.

This pure policy stores owned bounded identities and copied enum values; borrowed
lookups do not escape. No native allocation, callback, queue or retirement owner
is added. Existing ticket retirement remains the only capacity release boundary.
Policy storage drops with Slots after monitor shutdown; retained capacity is
intentional bounded reuse, not native retirement evidence.

Permanent regressions failed before their respective fixes:

- Manual USB Start returned Replace instead of Wait at capacity.
- Disabled stale selection returned Ready instead of None.
- ADB eligibility still reserved a sibling's slot despite the corrected plan.

The bounded matrix covers capacities 1..4, every occupancy 0..capacity and
automatic/suppressed/manual policy. It preserves original sibling tickets and
eligibility, and tests closing versus actually retired capacity. Additional tests
cover invalid indices, unchanged automatic priority and the daemon control adapter.

Offline Linux results: 52 display-related library tests and 73 monitor tests
pass. All ten changed production functions have >=93.75% executable-line coverage
individually (Rust 1.98.1, cargo-llvm-cov 0.8.7, LLVM 22.1.8). Locked syn 2.0.119
measures 37 functions across the changed policy and regression files, maximum 9.
Logs, source hashes and LCOV remain in owned work directory
`2026-10-26-t1267-t1270`. These are finite policy/adapter checks; physical backend
acceptance and native Windows limits remain as recorded in TODO.md.
