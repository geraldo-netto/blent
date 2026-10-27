# Offline crate notice supplements (T1280)

Added a repository-owned catalog and eleven unchanged notice texts recovered
locally from T1278 evidence. Eighteen exact crate versions receive supplements:
the seventeen previously rejected crates plus epaint_default_fonts, which retains
its separate nested font licenses. Each pin binds Cargo identity, normalized
manifest SHA-256, published VCS metadata/revision and original registry checksum.
Each notice retains its pinned upstream URL or same-revision donor archive/member
and its SHA-256. No new network transfers occurred.

`crate_supplements.py` prepares an owned temporary source copy only for selected
pins. It verifies identity before and after copying, adds notices under
`.blent-upstream-notices/`, and supplies provenance to the existing archive
workflow. Ordinary crate discovery remains unchanged. Both notice outputs and
augmented source archives contain the supplements; returned crate records include
per-notice provenance. Registry source/cache inputs are never written.

## Verification

- The permanent AccessKit regression failed against the previous collector with
  `no crate notice files found` before the fix. The final same test was also run
  against the previous committed collector in an isolated Python namespace.
- Fifteen T1280 tests and seventeen T1279 tests pass. Cases cover stale versions,
  wrong metadata/manifests/revisions, changed identity during staging, absent or
  corrupt texts, invalid paths, symlinks, missing/malformed/oversized catalog,
  unsupported schema, empty groups, reserved paths, retained font notices,
  success/error/cancellation cleanup, partial overlay failure and pin integrity.
- All 28 existing AppImage tests pass. Normal script-suite discovery includes
  `scripts/tests/test_crate_supplements.py`.
- Coverage.py 7.15.2 on Python 3.12.3 measured 100% executable-line coverage for
  each of six new production functions and changed `copy_crate`/`archive_crate`.
- The repository Python AST complexity checker measured 38 functions across
  the two production files and new test file; maximum 7, none above 9.
- Fresh offline collection extracted all 337 audited registry archives into
  temporary directories after checking each archive against Cargo.lock. All 337
  generated source/notice outputs passed, 18 received supplements, and every
  extracted source file's before/after hash remained identical. Each temporary
  tree was reclaimed. No empty notice group was accepted.

Evidence: [verification](artifacts/2026-10-27-t1280/verification.json), compressed
logs, per-crate fresh-collection results and complexity measurements in the same
directory. The metadata-only AccessKit fixture makes the regression independent
of Cargo cache availability. The full fresh-source check used the cached original
archives; this fixture is not claimed to represent an executable source tree.

```sh
python3 -m unittest discover -s scripts/tests -p 'test_crate*.py' -v
python3 -m unittest discover -s scripts/tests -p test_appimage.py -v
```

## Separate production selection finding

Checking the actual `rust_sources` Cargo query offline revealed a graph mismatch:
361 default-feature/all-resolve-node crates versus the audited release's 337
normal/build dependencies (49 additions, 25 omissions). The release shell builds
the host with native-audio/accessory-transport and then the GUI. Ten additional
selected crates still lack notices. T1281 retains the evidence and completion
condition for correcting selection and resolving genuinely required inputs.
T1280 fixes the pinned supplement mechanism and the audited inputs; it does not
claim a successful full AppImage build, deployment or legal-compliance audit.

## Ownership and input bounds

The context manager owns one temporary directory from allocation through recursive
copy, identity validation, overlay installation and the caller's archive work.
The caller borrows the staged path/provenance only inside that context. Success,
rejection, partial failure and Python cancellation settle the borrow before
synchronous directory removal; no staged path is returned in the persistent crate
record. Existing source trees and output ownership rules remain intact. Failed
later archive work can retain diagnostic output in the build-owned workspace,
as documented for T1279, but no successful record is returned.

Metadata reads admit at most 1 MiB and reuse the regular-file descriptor owner
introduced in T1279. Notice copying hashes in bounded chunks; immutable provenance
is retained only in output records. Staging disk use scales with one finite crate
tree at a time. There are no background workers, callbacks or shared mutable
catalog caches. Cleanup assumes filesystem removal is available; process/OS loss
can leave a temporary tree requiring owner cleanup. Local Cargo sources and the
repository catalog are trusted build inputs, not a hostile mutation boundary.
Tests establish these bounded failure cases, not unrestricted native safety or
mathematical leak freedom.
