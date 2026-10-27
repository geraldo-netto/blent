# T1026: session retirement includes original native evidence

Session retirement now observes original creators and every registered native completion separately from parent-task results. Pending cleanup retains admission even after the supervisor exits.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The independent capture contract accepts a native completion recorder. Clones
also retain original creation lifetime. Each native creator task/thread and
dependent resource must register its own actual completion before exposure or
another await; a creator handle alone is never task/thread join evidence.
Implementation stays outside the contract, using shared original proofs and a
separate read-only observer. Completed history is pruned across repeated restarts.

Session construction installs retained admission before calling the platform
factory and retains creation through the returned capture future. Supervision
observes parent tasks, remaining creator handles and every registered original
completion independently. An empty task set, parent result, panic, cancellation
or completed supervisor no longer grants native retirement. Pending cleanup keeps
admission and route retirement closed while unrelated sessions progress. Existing
failure reporting remains terminal; cleanup does not overwrite failures as success.

Two permanent behavioral regressions fail before the supervision fix with
`T1026 parent result released original native retirement` and
`T1026 dropped session lost original creator`. The reporting contract and plumbing
already exist in that red run; supervision still has its original parent-only
completion behavior. The same regressions pass after original native/creator
observation is required. Cases cover success, error, panic, cancellation, dropped
owners, real owned blocking creators, late native creation, independently held
original admissions and sibling capacity. Additional production construction tests
cover native exposure before constructor panic and route retirement after the
actual supervisor has exited. Six new normal-suite tests pass; no assertions or
existing regressions were removed, skipped or weakened.

Fresh locked offline native collection passes 758 host library and 453 binary
tests; seven pre-existing benchmark ignores remain. All 44 production functions
in capture contracts, retirement recording, session construction and supervision
individually exceed 80% line coverage (minimum 88.89%). Immutable source/function
manifest, owned native object/source attestations, raw LCOV and per-function
results are included. Native objects remain in
`/tmp/blent-t1026-evidence/coverage/target`. Four architecture boundary checks pass.

Source audit covers 75 production/test functions, maximum complexity 8, using
unchanged repository Sonar-rule definitions and cached pinned Rust parser.
Native SonarQube remains unavailable under T1019. Tool versions are included.

```sh
cargo test --offline --locked -p blent --lib t1026
python3 /tmp/blent-t1026-evidence/scoped_rust.py coverage
python3 -m unittest scripts.tests.test_host_contract_boundaries
```

This completes the portable reporting and supervision boundary. Linux core
resource registration remains T1028, auxiliary/probe registration T1029, and
shared activity propagation T1027. No native feature qualification is inferred
from interface readiness. Display policy integration remains T1015; no installed
app, live device/desktop or network setting changed.

</details>
