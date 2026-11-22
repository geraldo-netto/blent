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
