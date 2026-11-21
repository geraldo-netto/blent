# Open-item fixes — 2026-11-21

Owner authorized all 112 items marked open at the start of this pass. Existing blocked and declined dispositions remain unchanged. Work is ongoing; this report records completed increments, not completion of the backlog. Source tests do not imply native platform acceptance.

## T1596 — stable USB inventory targets

Unchanged inventory scans retain physical-key target IDs and revision, while refreshing opaque backend payloads. Visible metadata changes advance revision and reject stale Connect requests; removal/reappearance allocates a new ID. Capacity, duplicate-key and counter exhaustion failures leave the original inventory intact. Original live links retain ownership independently of inventory changes.

All four new regressions failed before the fix. All 32 broker-control tests pass afterward, including original authority, tombstone and native-retirement tests. The exhaustion regression now requests a genuinely new target instead of requiring an unchanged target to consume an ID. All five maintained production functions have 100% executable-line coverage (cargo-llvm-cov 0.8.7); all 16 measured functions in the new inventory and affected boundary-test files have complexity <=9 (project Rust syn 2.0.119 checker). The controller owns at most the configured 512 inventory entries; replacement temporarily owns one additional bounded inventory and a key index, released on success or error. No native resource is created, released or reused by a scan.

## T1549 — queued media replacement

Camera/audio panels retain one validated replacement request after consuming the inactive original backend. The original owner and exact native fence remain authoritative until settlement; only then can the saved request create/admit its successor. Frame maintenance runs for hidden panels, with repaint polling only while a request is pending. Stop drops the queued request without claiming early retirement. Admission failures consume the request once and preserve the error, including immediately completed fences.

Six original delayed-retirement regressions failed before the fix. Two additional immediate-retirement feedback regressions failed against the initial implementation and pass after correction. All 42 selected GUI tests pass, including hidden-panel progress, cancellation, admission identity, validation and idle repaint behavior. All 17 maintained production methods have 95.65–100% executable-line coverage; all 283 functions measured in affected files have complexity <=9 (project Rust syn 2.0.119). Each panel owns at most one validated settings value, released on cancellation, admission/failure or panel destruction; the preexisting backend cleanup sponsor retains native work after close. This does not grant a new target authority before original retirement. Native Windows acceptance remains tracked in T1271.

## T1599 — visible USB broker diagnostics

The Linux broker writes tracing diagnostics to standard error, which its on-demand launcher inherits. The permanent black-box regression starts the actual broker binary with private state/runtime directories and a failing fake helper; it failed before the fix because the warning went only to stdout. It now observes the warning on stderr and an empty stdout, then terminates and reaps that exact child. The existing subscriber-startup failure test also passes.

The maintained Linux entry function has 88.89% native executable-line coverage in the unit binary; the black-box regression additionally exercises the running production entry. All six functions measured across entry/fixture files have complexity <=9 (project Rust syn 2.0.119). The unchanged unsupported-platform entry is outside this change. The fixture owns its child and temporary files, reaps on assertion failure, and never invokes a real accessory helper or installed broker.
