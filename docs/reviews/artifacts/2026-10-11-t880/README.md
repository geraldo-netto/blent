# 2026-10-11-t880 — README

Fresh owned Rust coverage collection excludes stale cached objects and attests measured source bytes. A native contaminated-counter regression demonstrates the difference; no production behavior changed.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T880: fresh owned local Rust collection replaces reused-target exports. Permanent regression was added before the collector and failed because the required owned collector did not exist; the existing contaminated-counter reproduction is retained under T853. The native fixture compiles an old binary, changes the source, compiles/runs the current binary and compares actual LLVM exports: adding the cached object includes stale_only; the fresh owned collection does not. It also checks inherited target/profile isolation and bounded invalid selection, source edit, existing output, external object link, failed run and missing report rejection. Six final collector tests pass; the wider 86-test coverage-tool suite passed before the final additional failure-path test. Actual offline pinned Cargo collection ran all 281 common unit cases; all 14 trust functions pass strict unchanged native counters. Output objects/profiles remain in the owned temporary collection; compressed LCOV, source inventory, logs and SHA attestation are retained here. No dependencies downloaded, device tests or production behavior change. Development tool is exempt from production-line coverage; named complexity remains <=9.

</details>
