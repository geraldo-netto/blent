# Crate notice collection (T1279)

The AppImage collector now recursively preserves notice paths, including
`UNLICENSE`, nested `LICENSES` directories and epaint's OFL, UFL, Hack and emoji
font notices. Cargo's `license_file` is mandatory when declared, even when another
license is present. Empty inputs, missing files, escaping declared paths,
symlinks and special files reject packaging. Each successful crate manifest
records relative notice paths and SHA-256 values verified against regular members
of its corresponding source archive.

Discovery/copying lives in `packaging/appimage/crate_notices.py`; `sources.py`
retains Cargo/build orchestration. No network requests or new dependencies were
added. Filename discovery does not prove legal completeness. An offline audit
of 337 cached crates collected 320 and explicitly rejected 17 whose upstream
archives omit workspace licenses. T1280 tracks supplying pinned supplemental
inputs to future builds. T1278's installed recovery is not a build-input adapter.

## Verification

- Four permanent regressions failed against the old implementation before the fix:
  nested notices, declared paths, absent notices and missing declared files.
- All 17 T1279 tests pass, including invalid paths, symlinks, FIFO inputs,
  whitespace-only notices, partial-copy cancellation, descriptor failures,
  existing-output protection, archive mutation and streaming boundaries
  (1, 65535, 65536, 65537 and 131073 bytes).
- All 28 existing AppImage tests pass. The new file is included by the normal
  `scripts/coverage/script_check.py` discovery of `scripts/tests/test_*.py`.
- Coverage.py 7.15.2 / Python 3.12.3 measured 100% executable-line coverage in
  each of the nine new module functions and changed `sources.copy_crate`.
- The repository Python AST complexity checker measured 80 functions across
  both production files and both test files; maximum 7, none above 9. Individual
  results and tool/source hashes are retained; no SonarQube run is claimed.

Evidence: [artifacts](artifacts/2026-10-27-t1279/verification.json), with compressed
red/green/existing-suite logs, per-function complexity and the complete cache audit
in the same directory. No full release build, upload or installation was performed.

Reproduction:

```sh
python3 -m unittest discover -s scripts/tests -p test_crate_notices.py -v
python3 -m unittest discover -s scripts/tests -p test_appimage.py -v
```

## Ownership and bounds

The build owns its output workspace. Exclusive creation admits a notice directory;
cleanup never targets a pre-existing directory. Failure or cancellation during
copying removes that newly owned directory. Missing input rejects before output
creation. Existing archives are never overwritten. Later archive failure leaves
diagnostic artifacts owned by the build workspace, raises, and returns no success
record; this is not an atomic transaction across archive and notice outputs.

`regular_reader` owns one descriptor and its buffered reader borrows it. Regular
file checks occur before open and after descriptor admission; nonblocking open
prevents a raced FIFO from hanging admission. Context exit settles the buffered
borrow before exactly one descriptor close, including exceptions and cancellation.
Tests inject failed stream creation and nonregular post-open identity. Copying
uses 64 KiB chunks; selected-path storage scales with the finite crate tree.
There are no workers, callbacks, retained contexts or cross-call mutable state.

Inputs are local Cargo source trees and the workspace is owned by this build;
hostile concurrent filesystem mutation is not a supported isolation boundary.
Archive checks detect changed selected bytes or nonregular archive members.
Cleanup assumes filesystem removal remains available; abrupt process/OS loss can
leave workspace files for the build owner to reclaim. No unrestricted filesystem
race safety or mathematical leak-freedom proof is claimed.
