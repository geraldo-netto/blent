# T842 — strict current-toolchain Clippy

Strict Clippy checks now pass without suppressions or behavioral policy changes. Scoped native counters and existing media/lifecycle regressions support the refactoring.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Replace constant-size chunk iterators with array-chunk views, preserving the
existing remainder behavior; move audio wire tests after production items;
remove needless lifetime/borrow/clone expressions and doc-comment blank lines.
Camera orchestration reuses its existing `Session` context instead of passing
eight separate arguments. No lint suppression or behavioral policy changes.
The normal Linux build workflow installs Clippy with Rust and enforces the same
strict host/common command before building deployment binaries.

The original strict command failed; the same command now passes:

```sh
cargo clippy --offline --locked -p blent -p blent-config --lib --tests -- -D warnings
```

All 218 common library tests and 335 host library tests pass (three retained
opt-in benchmarks ignored). The changed T588 decoded-picture timing fixture also
passes separately. Existing malformed PCM, EDID and lifecycle regressions remain.
Fresh LLVM collections cover all nine changed production functions at least
95.83% individually; complexity remains at most nine. Moved methods retain native
counters. This is scoped verification, not a full-project coverage result.

Raw strict-lint failure/success, test output, native counters and per-function
results are retained alongside this file. These collections include the separate
T841 observer fix, which prevents its live-receive counter underflow; no counters
were dropped, clamped or otherwise repaired.

</details>
