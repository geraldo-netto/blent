# T841: retirement observer coverage snapshots

The strict reader correctly rejected the original LLVM export. This was a
coverage-expression problem while an observer remained blocked inside `recv`,
not evidence of a use-after-free or an actual timeout count of `u64::MAX`.

A minimal instrumented executable imports the actual `common/src/retirement.rs`,
creates one idle pool, and deliberately keeps it alive until process exit. Only
the logging macro is stubbed, so no external crates or downloads are required.
The original source exports `DA:145,18446744073709551615`. The same probe with
atomic LLVM counter updates exports exactly the same underflow: concurrent lost
increments are not required. No messages, adapters, or resource disposal run in
this reproduction. LLVM's inferred match-arm counters are inconsistent while
the preceding blocking receive has not returned.

`reap` now separates receiving from dispatching completed messages through
`reap_message`. The match counters are entered only after a receive completes.
The dispatch still retains records, polls on timeout, exits on disconnect, and
quarantines panicking adapters. No owner is joined on the caller's shutdown path,
no resource is abandoned early, and no coverage counter is clamped, filtered, or
excluded. Waiting for every observer before exporting would be inappropriate:
a quarantined resource can intentionally keep its observer alive indefinitely.

The permanent regression is
`scripts/coverage/test_retirement_counters.py`, in the existing coverage-tool
unittest suite. It invokes installed Rust/LLVM tools, strictly ingests the raw
LCOV, and requires evidence that the observer actually started. A missing
compiler/tool fails rather than skipping. The CI job now installs the same
stable Rust toolchain plus `llvm-tools-preview` used by native coverage jobs.
This run used cached tools only.

A permanent Rust test checks completed-message handling, delayed completion,
timeout, disconnect, and fail-closed panic quarantine. Existing retirement
lifecycle and invalid-capacity tests remain intact.

Validation on Rust 1.98.1 / cargo-llvm-cov 0.8.7:

- Strict native regression: original source fails, changed source passes.
- Fresh retirement-filtered Cargo run: 11 tests passed.
- Native Cargo/probe counters combined: all 12 production functions in
  `retirement.rs` have 100% executable-line coverage; maximum cyclomatic
  complexity is 6. Probe also exercises the public capacity accessor.
- Coverage reporting suite: 70 tests passed.

[Raw exports, logs, source fingerprints, and per-function results](artifacts/2026-10-10-t841/summary.json)
are retained. Coverage claims are scoped to this module, not the entire project.

Reproduce locally with already installed tools:

```sh
python3 -m unittest discover -s scripts/coverage -p test_retirement_counters.py
CARGO_NET_OFFLINE=true CARGO_LLVM_COV_TARGET_DIR=/tmp/blent-t841-fresh \
  cargo llvm-cov --offline --locked -p blent-config --lib --lcov \
  --output-path /tmp/blent-t841.lcov -- retirement
```

Use a previously unused coverage target directory for changed source. Set
`BLENT_T841_LCOV` to a file path to retain the strict probe's unmodified export.
