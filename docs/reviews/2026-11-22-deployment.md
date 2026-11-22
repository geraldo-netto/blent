# Local deployment of 080d6546

Owner requested completion of the current task, commit, reinstall and reload.
T1778 is committed as `080d6546` with 305 selected broker/task tests passing;
its 14 changed production methods meet the per-function coverage gate. See
[broker verification](2026-11-22-broker.md).

The current Linux host and GUI were rebuilt with the authoritative release
build units, including native audio and accessory transport. Both C helpers
were rebuilt. Existing native dependency binaries were retained. This is a
local extracted AppDir deployment, not portable-release qualification.

The Android release APK was rebuilt and verified against the existing official
signing certificate. It was installed with replacement/data preservation on
RugKing Pad 2 Pro `8002RH1010011900`; unrelated benchmark/profile applications
were left alone. Host registration and enabled autostart preference were retained.

After installation, all six project executable hashes matched staging. All five
live project processes (daemon, activity helper, broker, GUI, capture helper)
matched installed executable identities. The daemon and GUI were active. Android
reported successful cold launch and its main activity was resumed. The tablet
reconnected; H.264 VAAPI encoding started at 1280×800 and X11 touch/pointer mapping
completed. The tablet screenshot matched the orange host desktop output, with
the tablet settings overlay. This is startup/display verification, not new camera,
audio, direct-USB or sustained performance acceptance.

Installed notices now describe the actual selected Linux normal/build graph:
334 Rust package versions. Integrity checks covered 56 bundled files and 661
notice/metadata files. Build dependencies are included explicitly and are not
all claimed to be linked runtime libraries. T1463 remains unresolved: hex-slice
0.1.4 has original MIT declaration metadata but lacks an authoritative standalone
license notice. The manifest and visible index disclose this; full release-notice
qualification is not claimed. External runtime observations retained from the
previous manifest are labelled historical. T1734 remains open for its paused
native-audio dependency prerequisite and remaining integration.

[Installed identities and local rollback location](artifacts/2026-11-22-broker/deployment/installed.json)
record the exact binaries and APK. Previous AppDir and APK, build/install logs and
screenshots remain in that private local rollback directory. No repository changes
were pushed. The later evidence-only commit does not change deployed runtime code.
