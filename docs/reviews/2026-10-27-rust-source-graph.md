# Release Rust source selection (T1281)

Release source collection now follows each actual release build's normal/build dependency tree, avoiding development-only feature leakage. The offline collector produced 337 nonempty source/notice groups with verified supplements. Tests cover graph ambiguity and build failures; this validates collection and command selection, not a full rebuild or deployment.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

The collector now uses the normal/build dependency tree for each release build,
with exact package metadata supplying archive identities and source paths.
`rust_build.py` owns the two existing release invocations: host with native-audio
and accessory-transport, then GUI with default features. `build-release.sh` calls
that helper; commands, working directory and inherited target-directory settings
retain the previous native Linux release behavior.

The graph adapter queries each invocation separately with locked/offline Cargo
metadata and an x86_64-unknown-linux-gnu normal/build tree. It joins tree rows to
unambiguous metadata packages, requires the selected workspace root, excludes
workspace sources supplied by the project release, and unions external package
IDs. Each resulting crate record includes `build_manifests`. Duplicate, unknown
or ambiguous identities, external non-workspace path dependencies, malformed trees
and metadata changes between queries fail explicitly. The adapter intentionally
rejects ambiguous same-name/version identities instead of guessing their source.

Filtering metadata edges alone is insufficient: Cargo metadata can unify a
dev-only feature into a normal dependency. A permanent local Cargo fixture
demonstrates this condition and verifies that the normal/build tree excludes the
optional crate pulled only by that dev feature. No fixtures require network access.

## Evidence

The initial regression failed against the previous collector: dev-only and
dev-feature-leak crates were incorrectly collected. Eight graph/build regressions
now pass, including the real Cargo fixture, shared/repeated-edge permutations,
invalid rows, missing roots, ambiguous identities, source changes and release
builder failure propagation. Existing crate, AppImage and packaging suites are
retained. Evidence logs and exact counts are archived alongside
[per-function measurements](artifacts/2026-10-27-t1281/verification.json).

The actual `sources.rust_sources` entry point completed offline: 337 archives and
337 nonempty notice groups, with 18 supplemented crates. The selected identities
equal the previously audited release graph; all ten missing-notice crates from
the erroneous broader graph are excluded. Source archives retain notice hashes
and supplemental provenance checks from T1279/T1280. The check generated real
archives in an owned temporary workspace and reclaimed it after recording the
manifest. No new notice downloads were needed.

Cargo 1.98.1 and Python 3.12.3 were used. Coverage.py 7.15.2 measured 100% executable
line coverage in each of the ten new/changed production functions. The repository
Python/Shell complexity checker measured 74 functions across changed modules,
tests, release script and the collection probe; maximum 7. The release shell has
no function definitions; its changed helper invocation was exercised by existing
portable-release fixtures, including literal checkout paths and failure cases.

This validates Rust source/notice collection and the shared build-command contract.
No native rebuild, full AppImage build, device reinstall, deployment or upload was
performed. Counts describe the pinned current dependency graph, not a universal
claim about future Cargo versions or dependencies.

## Lifetime and bounds

Build options are immutable tuples. Metadata, tree text, indexes and the union
are call-local Python values with no retained global cache, callbacks or threads.
Only validated package metadata and selecting-manifest strings pass to archiving;
borrowed Cargo tree strings and temporary indexes do not escape the selection call.
Storage scales with the finite lockfile graph and Cargo's deduplicated tree output.
The adapter completes all queries/validation before the collector writes archives.

The existing synchronous command runner owns Cargo query processes and captured
output; errors propagate before collection. The release helper waits for each
build process and stops on failure, preserving the shell's completion marker rule.
Tests/mock children and the local Cargo fixture settle before temporary directories
are reclaimed. Archive/staging ownership remains as documented in T1279/T1280;
this change adds no native allocations, device handles or asynchronous lifetimes.

</details>
