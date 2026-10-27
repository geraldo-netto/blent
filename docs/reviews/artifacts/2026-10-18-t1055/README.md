# T1055: independent display activity signal contract

Independent display-activity hints retain nonwrapping revisions and stop fences without granting media or cleanup authority. Shared vectors and bounded traces validate the values before integration.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The [independent contract](../../../display-activity-signal.md) defines versioned
value hints scoped to the original authenticated control connection. Signed-range
nonwrapping revisions and remembered stop counters preserve coalesced activity;
unavailable observations deny eligibility. Unchanged observations emit nothing.
Exhaustion is terminal and requires the adapter to close that original lifetime.
Hints convey no credential, local consent, media permission or native retirement.
Rust projection/schema and Kotlin schema validation are implemented; production
publication/consumption remain T1056/T1057 and wake ownership T1058. T1011 retains
joined Android acceptance, T914 physical host/tablet acceptance.

Shared JSON vectors cover four valid and 14 invalid value cases, including exact
integer/Boolean types, missing/extra members, unknown versions, numeric overflow
and future stop fences. Both languages retain bounded constructor boundary tests.
The independent Rust model exhausts 1,024 five-command traces over ready, paused,
coalesced stop/resume and unavailable publications. Separate tests retain source
closure/absence, repeated no-op, counter maximum and terminal failure checks.
The containing projection owns two bounded value records plus one failure bit;
all inputs/outputs are value copies, with caller-bounded retained observations.
No native handle, allocator, queue or callback is introduced. Original native
cutoff/quiescence/cleanup/reuse remain independent adapter obligations.

Fresh offline LLVM collection passes all 356 common tests. All nine new maintained
Rust production functions independently measure 100% executable-line coverage.
Full ordinary Android collection passes all 2,004 tests using cached dependencies
and Android runtimes. All three authored Kotlin methods independently measure
100% native JaCoCo 0.8.15 line coverage. Source/vector hashes, native Rust objects'
hashes, raw LCOV/JaCoCo counters, class hashes, inventory and reports are retained.
Kotlin initialization is measured natively; supported compiler-generated methods
are excluded by the existing inventory, with no new exclusions or weakened tests.

Repository source audits cover 20 Rust production/test functions (maximum 6) and
eight Kotlin production/test/inline functions (maximum 6). Cached parser versions:
tree-sitter-rust 0.24.2/tree-sitter 0.25.2 and pinned Kotlin PSI 1.9.20, hash verified
before invocation. These are source estimates; native SonarQube remains T1019.
Twelve architecture/contract/portability checks pass. No network transfer, install,
upload, push, live lock/suspend/display change or tablet interruption occurred.

</details>
