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

## T1556 — current desktop button names

Conversion, encoder-worker, GPU and pipe-buffer help now names Save or Save & restart all displays, matching the actual controls. GPU wording makes its restart requirement explicit. Four existing settings tests pass; the pipe disclosure test now opens the help and verifies the Save instruction. All four maintained help-rendering functions have 100% line coverage; measured functions in affected files remain <=9 under the project Rust syn checker. This is text correction with unchanged settings behavior and resource ownership. Current guide corrections remain under T1591.

## T1519 — explicit connection-worker timeout failures

Own binding/startup timeout expiry becomes IOException, while caller cancellation and nested deadlines retain their original cancellation identity. Saved-trust observation has its own validated 1–60000 ms budget, default 5000 ms, independent of the startup budget. Expiry only ends observation; the original Binder, binding and blocked IPC jobs retain existing retirement obligations. No resource is released based on timeout alone.

All six original API-27/34 reproduction cases failed before the fix. All 84 selected Android/JVM cases pass after it, covering expiry, budget bounds, caller cancellation, nested timeout identity, trust operations, blocked start/stop and ResourceLane settlement. Existing retirement tests use the dedicated trust budget and IOException classification while preserving exact original-Binder/native-call assertions. All 11 maintained methods/callbacks have 100% JaCoCo 0.8.15 line coverage. All 339 functions measured in affected Kotlin production/test files have complexity <=9 (project Kotlin PSI 1.9.20 checker). These are host-JVM/fake-Binder observations; installed APKs and physical Android sessions remain unchanged.
