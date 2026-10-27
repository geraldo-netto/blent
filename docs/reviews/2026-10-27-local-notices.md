# Installed component attribution recovery (T1278)

The installed Linux bundle recovered component notices and a manifest tied to its exact executables and dependencies. Verification checks hashes, provenance and safe regular-file reads; replacement binaries invalidate that snapshot. This repairs one installation, not the production collector or a portable-release/legal-compliance attestation.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Restored `~/.local/share/blent/appimage/Blent.AppDir/usr/share/doc/blent/bundled`
without replacing executables, libraries, settings, drivers or the running service.
`index.html` provides offline component credits and links to full notices;
`manifest.json` records exact installed identities, attribution groups and sources.

The 56-file inventory covers AppRun and all files under usr/bin, usr/lib and
usr/libexec. Retained native dependency files match the cached release AppDir
byte-for-byte. Its 42 Debian component notices, FFmpeg/codec-header notices and
libevdi terms were recovered with source metadata. The six updated project
executables match the existing deployment receipt for cf521d43. The old AppImage
runtime is not part of this extracted installation and is not falsely attributed
as an installed runtime component.

The installed build's Cargo manifests/lock match the current dependency inputs.
Offline Cargo resolution with the recorded Linux native-audio/accessory-transport
features yields 337 normal/build dependency crates, excluding dev-only edges.
Each cached registry archive matches its Cargo.lock checksum. This graph describes
build dependencies as well as runtime dependencies; it is not proof every listed
crate is linked into every executable. Upstream notice files were read from these
verified archives, including nested font notices and UNLICENSE. Source archives
remain locally available by absolute path and checksum in the manifest.

Some crates omit workspace license files from their published archives. With the
owner's explicit transfer approval, nine missing text files were fetched from the
exact upstream commits recorded by those crates: 58,380 payload bytes total.
URLs, revisions and SHA-256 digests are retained in downloads.json. Shared egui
and AccessKit notices are associated only with crates at the matching workspace
commit; individual crate license expressions remain explicit. atspi-proxies uses
notices from the checksum-verified atspi-common archive at the identical upstream
commit. No packages or binary payloads were downloaded.

The original cached native source archive's manifest matches the release source
manifest; FFmpeg/header input hashes were also checked. The new manifest contains
1,988 notice/provenance files across 382 attribution groups. Fifty-one distinct
external runtime ELF files were identified through read-only loader resolution
under the installed wrapper environments and identity-checked process maps.
Flatpak SDK/runtime files and host-system/driver files are explicitly external,
not bundled Debian components. Unstarted plugins/features are not claimed observed.
This records provenance and notice integrity; it is not a legal-compliance or
portable-release attestation.

## Permanent verification and resource boundaries

`python3 scripts/verify-local-notices.py APPDIR` checks exact executable inventory,
notice hashes, attribution dependencies and explicit absence of unresolved inputs.
It failed against the missing directory before recovery, then passed after atomic
publication. Replacing executables or adding libraries invalidates this snapshot;
future local deployments must refresh and reverify attribution data.

Twelve permanent normal-suite tests cover absent/corrupt notices, unlisted binaries,
launcher identity, unsupported schema, missing attribution groups/dependencies,
unknown recovery state, path escape, duplicate/oversized input, special files and
CLI outcomes. A mapped DRM character device caused the initial runtime probe to
block; that exact owned inspector was interrupted and its exit confirmed by pidfd.
The repaired probe skips nonregular mappings. The permanent verifier's special-file
regression failed before regular-file admission was added and now passes without
attempting a device/FIFO read.

The read-only checker admits manifests up to 8 MiB and file inventories up to 8,192
rows. Hashing streams regular files with bounded buffers. One context manager owns
each descriptor: pre-open type validation, nonblocking open, post-open fstat type
check, borrowed buffered stream, and unconditional descriptor closure. Rejection
and stream-construction failure tests verify exact close ownership. No workers,
callbacks, sockets, mappings, installed temporary files or native operations remain
pending after verification. Notice publication used an owned temporary directory
and atomic rename; the current daemon retains PID 648049 with zero restarts.

All eight checker functions have 100% executable-line coverage with Coverage.py
7.15.2. The installed Python AST complexity checker measures 22 changed/test
functions, all <=9. [Verification and source hashes](artifacts/2026-10-27-t1278/verification.json)
and the [compressed installed manifest](artifacts/2026-10-27-t1278/installed-manifest.json.gz)
retain the evidence. Detailed work remains in the owned external directory
`/backups/disk2/blent-work/2026-10-27-t1278`.

T1279 separately tracks the production AppImage collector's top-level-only notice
scan and silent empty notice directories. Recovery corrects this installed state;
it does not claim that packaging follow-up is already implemented.

</details>
