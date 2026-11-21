# Open-item fixes — 2026-11-21

Owner authorized all 112 items marked open at the start of this pass. Existing blocked and declined dispositions remain unchanged. Work is ongoing; this report records completed increments, not completion of the backlog. Source tests do not imply native platform acceptance.

## T1596 — stable USB inventory targets

Unchanged inventory scans retain physical-key target IDs and revision, while refreshing opaque backend payloads. Visible metadata changes advance revision and reject stale Connect requests; removal/reappearance allocates a new ID. Capacity, duplicate-key and counter exhaustion failures leave the original inventory intact. Original live links retain ownership independently of inventory changes.

All four new regressions failed before the fix. All 32 broker-control tests pass afterward, including original authority, tombstone and native-retirement tests. The exhaustion regression now requests a genuinely new target instead of requiring an unchanged target to consume an ID. All five maintained production functions have 100% executable-line coverage (cargo-llvm-cov 0.8.7); all 16 measured functions in the new inventory and affected boundary-test files have complexity <=9 (project Rust syn 2.0.119 checker). The controller owns at most the configured 512 inventory entries; replacement temporarily owns one additional bounded inventory and a key index, released on success or error. No native resource is created, released or reused by a scan.
