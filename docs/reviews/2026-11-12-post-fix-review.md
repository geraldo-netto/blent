# Post-fix project review — 2026-11-12

Reviewed baseline `f0dda8f1` across shared contracts/storage, host connection and
media ownership, Android display/connection/consent, desktop presentation, C
capture/USB helpers, installation, packaging, CI and current documentation.
This is a cross-domain source review with targeted reproductions, not a claim
that every source line or native platform was exercised. Generated files,
dependencies and historical evidence were not treated as current implementations.
No production behavior or permanent tests were changed.

Six findings are recorded in [TODO.md](../../TODO.md). Existing blocked native
acceptance and display-route ownership items remain unresolved; this review does
not reopen declined stylus/macOS work or authorize physical mapping trials.

| Finding | Confirmed outcome | Evidence |
| --- | --- | --- |
| T1370 | Source autostart accepts a directory as its destination; AppImage autostart accepts directories and directory symlinks. The staged entry is nested inside instead of replacing the intended file. | [Source autostart](artifacts/2026-11-12-post-fix-review/autostart-directory.json), [AppImage registration](artifacts/2026-11-12-post-fix-review/appimage-registrations.json) |
| T1371 | Source and AppImage service installation accept directory destinations. AppImage also leaves staging behind after renderer failure. | [Destination probes](artifacts/2026-11-12-post-fix-review/installer-destinations.json), [registration probes](artifacts/2026-11-12-post-fix-review/appimage-registrations.json) |
| T1372 | AppImage image/entry publication reports success for directories and directory symlinks and selects an unusable launch path. | [Destination probes](artifacts/2026-11-12-post-fix-review/installer-destinations.json) |
| T1373 | AppImage desktop registration reports success for directory destinations; renderer failure leaves staging behind. | [Registration probes](artifacts/2026-11-12-post-fix-review/appimage-registrations.json) |
| T1374 | Native Linux `wifi --off` reports success after failing to acquire the endpoint lease, with zero ADB invocations. | [CLI probe](artifacts/2026-11-12-post-fix-review/wifi-off-locked.json) |
| T1375 | Authenticated USB tablets lose available model/manufacturer names when projected into the resource selector. | Source trace below; no rendering or physical-device reproduction claimed. |

## Reproduction boundaries

Installer probes source the actual shell function definitions, excluding their
top-level installation calls. Every destination belongs to a fresh temporary
directory. Directory cases contain an existing `retained` file; symlink cases
point to a separate directory in that same temporary tree. The production
function is called directly and its exit status and staging location recorded.
Service/desktop renderers are substituted with bounded shell fixtures, and
`systemctl` is stubbed: no service or real user installation is modified.

For source autostart, call `install_autostart_entry printf "%s\n" "[Desktop Entry]"`
with `CONFIG_BASE` set to the fixture. For source service publication, substitute
`write_user_service` with a successful renderer and call
`write_installed_user_service`. AppImage image publication copies a regular
fixture file; entry publication uses `LAUNCHER="$DEST/Blent.AppImage"`.
AppImage registration probes call `install_service`, `install_desktop` and
`update_autostart` with the service-enabled check stubbed successful.

The AppImage `autostart/renderer-failure` case is a successful control: the
service-enabled branch uses `cat`, so the failing renderer fixture is not called.
It is not evidence of a renderer failure. The service and desktop failure cases
do invoke their failing renderers and leave staging files.

The Wi-Fi probe uses the freshly built production CLI, a private configuration
with `wifi_address = "192.0.2.99:5555"`, a private runtime directory and a fake
ADB executable that records calls. Python holds an exclusive `flock` on the
production endpoint stripe file while executing `blent wifi --off`. The command
returns 0 and clears the saved address; no ADB call occurs. This demonstrates
failure to admit cleanup, not a physical transport-state measurement.

[Source hashes and executable identity](artifacts/2026-11-12-post-fix-review/manifest.json)
bind these observations to the reviewed code. Each finding specifies permanent
regressions required before a later behavioral fix.

## Cross-domain checks

- Shared contracts/storage: examined trust admission, certificate-bound storage,
  pairing epochs, snapshot validation, tablet identities, workspace merging and
  configuration transaction ownership.
- Host: traced Wi-Fi admission/rollback/exit, native endpoint leases, registered
  display ownership, capture-generation and pause fences, input-controller
  replacement, camera frame/output ownership, audio retirement and raw mappings.
- Android: traced route replacement, token delivery, control callback identity,
  display generations, resource retirement, direct-USB presentation and local
  camera/audio consent. No new physical black-screen diagnosis is claimed.
- GUI: checked discovery/connection presentation, responsive device rows,
  status sampling, media admission and resource selection. For T1375,
  `Panel::tablets` constructs `Device` using `identity.label()` without reading
  `Snapshot::devices().names`; `tablet_choices` renders that projected name.
  `Device::merge` also chooses names by presence and lexical order, so any fix
  must preserve descriptive names against generic labels from other sources.
- Native C: examined raw-ring allocation, packet/FD handling, generation changes,
  frame-exchange publication/retirement and USB-debugging reservation. These
  source checks are not native safety certification.
- Tooling/docs: checked source/AppImage installers, renderers, Arch packaging,
  build workflow, and current installation/security/Windows guidance. Retained
  Debian files are explicitly historical regression fixtures, not an active
  release path; they were not reported as compatibility code or version drift.

The preceding fix pass's Rust, targeted Android, Clippy, complexity and Windows
cross-compilation evidence remains a baseline observation; it was not rerun or
counted as fresh testing here. Native Windows counters, physical USB acceptance
and optional PipeWire build prerequisites remain outside this pass's evidence.
Fresh validation: 549 of 550 discovered script tests passed, with one existing
native-Windows process-ownership test skipped and zero failures. The production
host build passed. All 19 TODO IDs are unique; the 13 Other-item rows are unchanged.
[Verification and logs](artifacts/2026-11-12-post-fix-review/verification.json)
record the commands, outcomes and exact skip.
