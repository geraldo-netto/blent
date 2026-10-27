# T1052: explicit idle retention and test-only thread observation

Idle retention fields and test-only observation are now explicit without changing runtime cleanup. Existing held-owner regressions and fresh scoped counters remain.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The production compilation retained under T1049 reports two warnings: the idle
Handle's deliberately retained `lease` field is never read in production, and
Worker's `is_finished` is called only by tests. The field is now `_lease`, with
its retention purpose documented. The thread-exit observer is explicitly
`#[cfg(test)]`; its body and every permanent caller/assertion remain intact.
No runtime behavior or native cleanup transition changes.

The Handle still owns one Arc view of the original lease, in the same field
order. Handle destruction revokes the worker first; the lease view drops before
Worker transfers its actual thread and original retained lease into the existing
native retirement owner. Thread exit alone remains insufficient: the reaper joins
that original thread and observes original lease cleanup. Pending work keeps its
admission and cleanup sponsor. Rejection, cancellation, panic and held cleanup
continue through the same owners. Timing storage retains its existing 256-entry
bound. No allocation, callback, ownership handoff or reuse permission is added.

Offline accessory-enabled production compilation is warning-free. All 19 normal
idle lifecycle tests and four joined encoder/reader retirement regressions pass,
with no skips or weakened assertions. They include blocked native creation,
cancelled startup, worker panic, stale leases, held native cleanup and retirement
retry after original reader join. This is maintenance, not a behavioral bug fix;
the existing permanent boundary/fault coverage is retained without artificial
warning-only tests.

Fresh native LLVM collection runs the normal idle tests. The changed
`Startup::finish` and all nine production worker methods/functions independently
measure 100% executable-line coverage. Source/object SHA-256 attestations, raw
LCOV, collection log and per-function results are retained. The unchanged
repository source audit covers 40 scoped production/test functions, maximum
cyclomatic score 5, using cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2.
These source scores are not native SonarQube certification; T1019 remains open.

`scoped_rust.py` uses the cached grammar under
`/tmp/blent-t1049-evidence/parsers` and writes exclusively owned native output
under `/tmp/blent-t1052-evidence`. No downloads, installation, uploads, pushes,
live display activity or physical device tests occurred. The scoped hosted
checks do not establish unrestricted native safety or new platform support.

</details>
