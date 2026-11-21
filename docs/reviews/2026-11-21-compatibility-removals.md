# Compatibility-only removals — 2026-11-21

Scope: current Blent production, tools and packaging. The no-backward-compatibility rule is saved in local AGENTS.md, persistent maintainer preferences and CONTRIBUTING.md. Hardware capability selection, current recovery and historical reports are distinct from obsolete caller support. This audit is in progress; installed sessions are unchanged during source verification.

## T1534 — position aliases

Removed undocumented `top`/`bottom` aliases. Current values remain right/left/above/below; the parser rejects obsolete aliases, and persisted invalid values follow the existing invalid-value policy. The permanent rejection regression failed before removal. All 64 selected model/storage/runtime cases pass; the maintained parser has 100% executable-line coverage. The parser and changed regressions have complexity <=9 (Rust syn 2.0.119 project checker; Rust 1.100.0 and cargo-llvm-cov 0.8.7). The parser borrows call-local text and returns an enum; no native resource or retained state is added.

## T1582 — indexed capture FIFO

Removed the instance-zero exception: every display now uses `capture-<instance>.fifo`. Current helper and status paths already follow the supplied FIFO; no old-name lookup, migration or deletion is added. Updated the doctor/monitor/selection fixtures that assert the actual runtime path. Independent fixtures that supply arbitrary FIFO paths remain valid current-interface tests.

The permanent isolated regression failed on instance zero before removal and now covers zero, ordinary slots and u32::MAX. All 64 selected shared cases and six host doctor/monitor/selection cases pass. The maintained path constructor has 100% line coverage; all six affected constructor/test functions have complexity <=3. The returned PathBuf is caller-owned; existing private-directory validation and FIFO lifetime ownership are unchanged. No running FIFO was renamed or removed.

## T1644 — decoder replay source adapters

Removed bridges for absent decoder profiles, socket input, metrics, selection receipts, retirement and old render callbacks. Selected revisions must provide the current dependencies and hooks. Replay callbacks preserve the production acknowledgement identity without rewriting. The complete dependency set includes GenerationFence for NativeDeadline.

The missing-dependency rejection regression failed before removal. Eight generator/dependency tests pass; the generated current-source APK compiles successfully (33 Gradle tasks). Fourteen Python functions have measured complexity <=4 (project Python AST checker); benchmark tooling is exempt from production coverage thresholds. Generated sources are owned by the caller's requested directory, and subprocesses are awaited; failure leaves reviewable artifacts without starting a device session. This verifies compilation, not device performance.
