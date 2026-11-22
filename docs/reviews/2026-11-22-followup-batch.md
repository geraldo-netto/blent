# Selected follow-up implementation

Owner scope: T1740, T1710/T1711/T1625, T1529/T1777/T1772, T1495 and T1795.
One verified local commit per item. No push, installation or reload is authorized
by this batch; installed sessions remain untouched. T1740 is decomposed into
T1808 toolchain and T1809 GUI migration with parent integration retained.

## T1710 — canonical service template

`scripts/blent.service` is the sole authoritative unit template. Arch packaging
uses the same literal-path renderer as source and AppImage installation; portable
staging no longer ships a duplicate. A changed-template fixture proves the package
consumes the canonical policy and checks rendered bytes and file mode. Both new
regressions failed before removal/consolidation and pass afterward. Existing
cleanup/ADB policies and path assertions remain intact. The affected six
source-build/user-path tests pass after the separate T1810 fixture correction.

The maintained package function executes all 23/23 measured shell lines. No native
session or new retained process owner is created: package() writes only within
its caller-owned staging directory. Native retirement fixtures use private units;
installed Blent services are untouched. Broader installation integration remains
T1625. [Evidence](artifacts/2026-11-22-followup-batch/t1710/).

## T1711 — shared privileged setup

Source/tarball, Make and Arch hooks delegate to `scripts/setup-system.sh`. The
canonical resource templates are staged with it. Existing EVDI configuration and
live devices are preserved; owned uinput rules refresh consistently. Configuration
write failures stop activation; GPU, udev and safe EVDI activation failures warn
without discarding installed configuration. AppImage user registration stays
unprivileged. There is no retained Debian package hook in the current tree.

New route/behavior regressions failed before consolidation. All 58 focused
installation tests pass afterward, including existing capacity, GPU ordering and
no-unload tests. All 11 maintained shell functions have 100% executable-line
coverage, using exact source hashes or body-hash-attested fixture extraction.
Complexity is at most 9 across the changed source/test files; measurement uses
the repository tree-sitter checker and Bash DEBUG counters. No native root setup
was performed. Temporary config publication owns one same-directory file until
synchronous install/link settlement; EXIT removes that exact temporary, and
no-clobber linking preserves concurrent external ownership. No background owner,
retained callback or unbounded buffer is introduced. Fault tests cover copy
failure, existing symlinks and concurrent publication.
[Evidence](artifacts/2026-11-22-followup-batch/t1711/).

## T1808 — packaging compiler prerequisite

Pin official `rust:1.100-bookworm` at manifest-list digest
`ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea`,
verified through Docker Registry's image manifest. Official amd64 source revision:
`rust-lang/docker-rust` `3f4c9cd699d3671ee657c75b5e592f6eb0b93824`. The complete
packaging image builds; its native compiler/linker probe runs with Rust 1.100.0,
glibc 2.36, installed EVDI 1.15.0, libusb 1.0.26 and PipeWire 0.3.65. This preserves
the Debian 12 ABI baseline while meeting eframe 0.37's Rust 1.95 minimum. The
permanent pin/minimum/documentation fixture failed at 1.90 and passes afterward.
This is toolchain qualification, not a full application or distribution build.
No production function or native session ownership changes.
[Evidence](artifacts/2026-11-22-followup-batch/t1808/).

## T1625 — installation integration

All current routes consume the canonical service template and shared privileged
setup where applicable. Added permanent Arch resource-byte checks and portable
bundle checks for the shared setup and absence of a duplicate service template.
The combined run passed 109 executed tests across source, Make, package staging,
AppImage and setup fixtures; one mistyped module name failed discovery, then the
actual identity module passed separately. All four artifact/parity tests also
pass with the new assertions. No assertion was removed. Changed test functions
pass complexity <=9. These isolated fixtures cover files, outcome policy and
private native-loader behavior; no pacman transaction, privileged setup or live
service reload was performed. Existing native module acceptance remains separate.
[Evidence](artifacts/2026-11-22-followup-batch/t1625/).

## T1740 / T1809 — evaluated candidate, not landed

The eframe 0.37 adapter candidate passed all 304 GUI unit tests, both process
lifecycle integration tests and the startup/accessibility integration test. A
further 16 native tests passed after explicitly retaining Wayland client
decorations. The release build and GUI Clippy pass; all 16 maintained production
functions have >=80% measured line coverage, and 618 measured source/test
functions have complexity <=9. Exact LLVM evidence combines the two executed
GUI objects with identical maintained source and differing feature selection.

The candidate handles the new separate logic/UI callbacks, including hidden
activation and terminal-preview cleanup (both new regressions failed before that
adaptation). Headless tests explicitly consume texture deltas. Fixtures retain
their original safety assertions: framework theme initialization is settled,
slider numeric text is located separately from its suffix, animation preferences
are set after selecting the test theme, and the notice-only viewport fixture
starts with no unrelated draft. An intermediate run rebuilt its executable during
self-spawning tests; those invocation failures were eliminated using an isolated
coverage target. Final tests do not exhibit those failures.

**The candidate does not satisfy the dependency cleanup objective.** In the
Linux GUI normal/build closure, distinct package/version pairs rise from 309 to
340; package names with multiple versions rise from 12 to 14. The old AccessKit
hashbrown 0.15 edge disappears, but the newly enabled Wayland clipboard data-control
path introduces `arboard → wl-clipboard-rs 0.9.5 → tree_magic_mini 3.2.2 → petgraph
0.8.3 → hashbrown 0.15.5`. Registry checks confirmed current petgraph/tree_magic/
wl-clipboard releases retain that requirement; arboard 3.7 retains it as well.
The selected API migration was therefore not landed. Current GUI dependencies
and code remain in place; T1740/T1809 retain the precise upstream prerequisite.
No staged bundle notices were relabelled for an unshipped candidate.

[Graph and recoverable patch identity](artifacts/2026-11-22-followup-batch/t1740/)
and [candidate validation](artifacts/2026-11-22-followup-batch/t1809/). Installed
processes and artifacts remain untouched. The packaging compiler improvement
T1808 remains independently verified and committed.

## T1811 — remaining Rust installation fixtures

The common test build exposed two compile-time references to the deleted unit
copy; the real-Cargo Arch fixture also failed because it omitted shared setup.
Both policy fixtures now read the canonical unit, retaining all deadline, kill
escalation and scheduling assertions. Arch stages the actual canonical renderer,
unit and setup script; its original fresh-output assertions pass for environment
and Cargo-config output overrides. Two policy tests and both Arch tests pass.
Changed test functions pass complexity <=9; no production function changed.
[Evidence](artifacts/2026-11-22-followup-batch/t1811/).
