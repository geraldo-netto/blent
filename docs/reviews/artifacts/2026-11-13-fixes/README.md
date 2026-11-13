# Review-finding fixes — 2026-11-13

Resolved separately: T1378 stale portable identity authority (`79910d13`), T1380
Linux endpoint identity revalidation (`10f54e0d`), T1379 obsolete Linux CLI alias
(`60c72590`), and T1381 independent portable discovery observations (this commit).
Each resolved row is removed from TODO.md. The 14 existing blocked/declined rows
remain unchanged from review commit `2c81e10e`; T1382 separately retains the
native Windows acceptance obligation from T1381. There are no open rows and
15 Other-item rows (11 blocked, four declined).

Behavioral fixes retain permanent failing-before/passing-after regressions in
the normal Rust suites. The compatibility-only module removal passes 430/430
common-library tests and has no remaining exact `linux::cli` or `super::cli`
references in common/host/GUI Rust sources. T1381's final integrated selections
pass 99/99 host cases across four test invocations. Per-item directories contain
red/green logs and measured function coverage where behavior changed.

The final project complexity check measures 28,921 functions, with zero above
nine. Rust formatting and Linux workspace Clippy with warnings denied pass.
Windows GNU cross-compilation passes with existing unused-code warnings in
control_outcome and FailureStage; it does not establish native Windows behavior
or counters. No Android installation, physical-device trial, push or deployment
was performed. Existing native acceptance and decision blockers remain in TODO.

`verification.json` and compressed logs record the commands/results. Earlier
per-item coverage artifacts describe their own commit; T1381's measurements
supersede the earlier portable observation implementation after its refactor.
