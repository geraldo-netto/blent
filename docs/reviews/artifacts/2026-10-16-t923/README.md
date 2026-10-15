# T923: bounded Linux display status

Linux snapshot reads, including destructor ownership checks, consume at most
65,537 bytes and reject anything exceeding 65,536 before JSON decoding. The
portable assignment-batch boundary accepts at most four individually valid
records with unique slots 0–3. Its sequence visitor stops before decoding a
fifth record's fields. The publisher validates before changing retained state,
sorts valid assignments, writes a private temporary file, and restores its
previous state if atomic publication fails. PID/start-time liveness checks stay
in place; descriptive status never grants connection authority.

Permanent normal-suite `t923` tests cover exact byte boundaries, actual read
budgets, excessive/duplicate records, all bounded slot subsets, invalid ports
and slots, every truncated prefix, malformed data, stale owners, publication
rollback/retry and destructor preservation of invalid replacement files. Before
the fix, three regressions fail: oversized valid JSON, duplicate snapshot slots
and duplicate publication. The same tests pass after the fix. No tests were
removed, skipped or weakened.

All 325 common library tests pass outside the sandbox. The sandboxed run passes
323, but rejects `SO_PASSCRED` in two unchanged Linux packet fixtures; the native
run preserves both fixtures and their assertions. Offline host checking with
`accessory-transport` and workspace formatting also pass.

Fresh native coverage uses cargo-llvm-cov 0.8.7 and an exclusively created target
tree, locked/offline common library selection, original source fingerprints and
owned-object attestation. All 26 production functions in `runtime.rs`,
`tablets.rs` and `tablets/batch.rs` exceed 80% executable-line coverage; every
changed boundary function reaches 100%. Raw native objects remain in
`/tmp/blent-t923-evidence/coverage-a/target`; compressed LCOV, fingerprints,
attestation and per-function results are retained here.

The scoped source audit checks 56 functions, including permanent tests, with
maximum complexity 9. Tree-sitter 0.25.2 is reused from its existing local cache;
the pinned Rust grammar 0.24.2 is compiled from the cached Cargo crate. The
retained `scoped_rust.py.gz` initializes only that Rust parser and executes the
repository's unchanged counting/inventory definitions; C, Shell and Kotlin are
outside this scoped audit. This avoids installing or downloading unavailable
Python parser packages. It is independent Sonar-rule source counting, **not a
native SonarQube measurement**; that missing evidence remains T1019.

Commands (all offline):

```sh
cargo test --offline --locked -p blent-config --lib
python3 /tmp/blent-t923-evidence/scoped_rust.py coverage-a
cargo check --offline --locked -p blent --features accessory-transport
cargo fmt --all -- --check
```

No live monitor, tablet, suspend, service or boot configuration was changed.
