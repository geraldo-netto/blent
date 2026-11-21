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

## T1645 — obsolete installer mode

Removed `--binaries-only`; prebuilt-bundle tests use `--user-install`. Unknown flags, missing/empty destinations and extra arguments are rejected before installer effects. No arguments still run the full installation sequence. The rejection regression failed before removal on the obsolete flag, unknown flag and extra arguments. Nine build/install tests pass. Native Bash tracing reports 18/18 executable lines covered in `main`; four affected functions measure complexity <=5 (project shell/Python checker). Tests use private temporary roots and command fixtures; no installed session or system configuration was changed.

## T1626 — Debian release remnants

Removed the obsolete Debian package directory and unused fakeroot/dpkg-deb test commands. Current build prerequisites and uninstall guidance no longer advertise Debian release packaging. Debian package-owner lookup and corresponding-source downloads remain required for AppImage dependencies. The documentation inventory marks its original Debian entry as removed while retaining historical metadata.

Retargeted the three EVDI safety regressions from the removed postinst to the current full system-setup entry, preserving all failure, Xorg deferral, capacity and no-unload assertions. No obsolete implementation remains in fixtures. All 32 setup/package/release/notice tests pass; seven affected test functions have measured complexity <=7. No production executable function changed. Fixtures own their temporary roots and mocked command traces; no kernel or installed package mutation was performed.

## T1646 — dormant cursor adoption

Removed the Cinnamon existing-device parameter and snapshot adoption path; only a newly added device can acquire the policy. The retargeted permanent regression failed before removal by adopting an old same-name device. It now verifies the new identity survives old-device removal and releases every signal/timer on its own removal. Existing foreign-device, expiry, replacement and multi-device assertions remain.

The executable JavaScript lifecycle model and both private Rust cursor tests pass. Rust `script` coverage is 100%; native Node/V8 counters cover every executable statement-start line in all seven policy functions, with nested callbacks measured separately. ESLint 10.15.0 classic cyclomatic scores are <=5 for all 25 policy/fixture functions; the project Rust checker reports <=4 for the two changed Rust functions. This is isolated lifecycle evidence, not a new physical Cinnamon acceptance claim.
