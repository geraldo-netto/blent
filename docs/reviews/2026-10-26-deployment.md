# Linux and Android local update — 2026-10-26

Updated the installed Linux applications and RugKing Pad 2 Pro Android application
from the latest local application code, `65f636fc`, with tracking-only HEAD
`5c1986a9`. Both retain version 1.2.3 (Android version code 12). No upstream
version claim or network fetch was made. T1268 tracks the requested host UI review.

Linux release builds used the cached offline SDK/runtime already used by this
installation, with `blent/native-audio,blent/accessory-transport`. Both C helpers
were rebuilt from current source. The existing libevdi and kernel setup were
preserved. These binaries depend on the installed local SDK paths; this is a
local deployment, not a newly verified portable release.

Android `assembleRelease --offline --no-daemon` passed, including release lint
and shrinking. The existing and replacement APKs both passed the designated
certificate/package verifier. Installation used `adb install -r -g`; application
data was retained. Pulling the installed replacement confirmed byte identity
with the verified build. Temporary signing properties were deleted afterward.

The old Linux service and its children exited before replacement. All six
installed executable hashes match the staged builds; all five running Blent
process images match the corresponding new executables. The updated GUI is
running. The service remains active with zero restarts, the tablet activity is
foreground, and successive tablet render acknowledgments establish live display
delivery at the configured 1280×800 mode. This smoke check does not establish
camera/audio acceptance, sustained performance or T1258's mapping-lifetime trials.

Touch and pointer both mapped to DVI-I-2-1; the touch transformation matches that
output's desktop geometry. An aggregate X11 mapping warning also occurred before
this deployment: the pen remains classified by Xorg as a keyboard. Preserve the
existing declined pen disposition T592. Missing KDE services on this Cinnamon
session do not negate the independently checked X11 mapping. The daemon reported
no StatusNotifierWatcher at this restart; its service and GUI still ran normally.

## T1269: obsolete local entry points cleaned up

The inactive `~/.local/share/blent/appimage/blent` wrapper still launched a
September `Blent.AppImage`, while PATH, desktop and systemd used the updated
extracted AppDir. Checked the desktop, autostart, user-service and shell startup
references, then moved that obsolete wrapper and AppImage into the rollback
backup. Active host and GUI launchers resolve to the current AppDir. Installed
packaging metadata and tracked distribution documentation were refreshed.

Configuration and EVDI-library hashes were unchanged by installation. Existing
host and Android release checks were already disabled and remained so. Old
executables, APK, metadata, documentation and obsolete entry points are retained
under `/backups/disk2/blent-work/2026-10-26-deploy/backup`. Device-side inspection
files and temporary signing credentials were removed. No global cache purge,
driver change, repository fetch/push or dependency download was performed.

T1269 is resolved by deployment-state cleanup; no production behavior changed.
Automated deployment assertions checked executable/APK hashes, active service,
render acknowledgments, touch/pointer mapping, current launcher targets,
archival of obsolete entry points and signing-file removal. Prior application
regressions remain intact; T1267's previously observed intermittent test failure
remains open rather than being treated as resolved by this smoke check.

Build logs, executable identities and verification results are in
[the deployment artifacts](artifacts/2026-10-26-deployment/verification.json).
