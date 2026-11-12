# Project follow-up review — 2026-11-12

Reviewed baseline: `ab1c33e6841c4ea97bdf23f7a3614102733e916c`.
Seven confirmed findings are recorded as T1358–T1364 in [TODO.md](../../TODO.md).
This review changes the ledger and evidence only. Production fixes and their
permanent failing-before/passing-after regressions remain open.

## Scope and limits

The review covered the project's major domains through source, contract, caller,
test and documentation checks. It was a boundary-oriented source review, not a
line-by-line certification of every function. Generated outputs, dependencies,
copied notices and archived experiments were not treated as maintained production
code. Existing blocked and declined work was preserved.

| Domain | Boundaries examined |
| --- | --- |
| Shared contracts and settings | FileConfig validation/sanitization, transactional persistence and merge/diff, release versions, credentials, camera/audio profiles, connection identity/management/window records, control snapshots and resource retirement |
| Linux host | Display authentication and ADB credential delivery, activity-source recovery, configuration negotiation, encoder geometry and acknowledgements, camera/audio framing and output ownership, Wi-Fi creation/rollback and address leases |
| Android | Session/control/video route changes, token startup/revocation, decoder packet bounds, media worker and ResourceLane retirement, wake ownership, audio framing, camera buffer/rate ownership, input translation and update checks |
| Desktop UI/platforms | USB identity, reservation and row-state projection, responsive columns, settings-save orchestration, Windows status adapter and explicit preview limits |
| Native C | USB discovery/reservation/open/stream boundaries, EVDI raw ring and frame exchange allocation/borrows, GPU option and encoded-output limits |
| Installation/build/tooling | Source/AppImage installation, desktop/service writers, bundle staging and Arch recipe, build/portability/complexity workflows, native coverage inventories and worker prerequisites |
| Documentation | Current architecture, installation, development, compatibility and platform guidance; retired tracking references; local file-link existence |

Recent broad verification from the preceding fix batch is retained in
[the tablet-recovery verification record](artifacts/2026-11-12-tablet-recovery/verification.json).
Those executions have their own source revisions and scopes; they are not new
runs or proof of complete current coverage. This review did not repeat those
unchanged full suites. No Windows runtime, physical USB mapping trial, tablet
reinstall, live connection interruption or publication was performed.

## Findings and evidence

| ID | Confirmed observation | Evidence and required fix validation |
| --- | --- | --- |
| T1358 | The exposed Linux tokenless mode cannot start Android video with an empty token store. Host delivery supplies no credential; RemoteDisplay refuses a null credential even with a valid surface. | The existing host tokenless regression passes (1/1 selected test). The existing RemoteDisplay lifecycle regression passes (1/1 selected test) and explicitly asserts zero worker requests after start plus surface with no token. These independent tests confirm opposite boundary policies; neither tests the joined configuration. Add the missing joined startup/transition regression before fixing the contract. |
| T1359 | The bitrate clamp diagnostic claims measured USB capacity for both an upward and a downward policy clamp. | `common/src/model.rs` defines the range as policy, then logs transport insufficiency for every out-of-range value. Inputs 0 and 60001 reach that branch. Correct wording and verify both boundaries; this is not evidence of a measured throughput defect. |
| T1360 | The CI release command omits the feature required for the USB broker advertised in its upload manifest. | Cargo metadata confirms `required-features = [accessory-transport]` and no host default features. The release command only enables native-audio. Earlier checks and temporary coverage targets do not supply this release binary. This is manifest/source evidence, not a remotely executed CI failure. Add a clean-target artifact regression. |
| T1361 | Current guides assign remaining acceptance to retired T857/T846/T737 tracking. | `git show 7827286f -- TODO.md` records retirement without completion. Preserve that distinction in compatibility/architecture prose; no native campaign is reopened. |
| T1362 | A desktop-entry writer failure truncates the existing launcher and returns success. | Execute the actual installer function with main disabled, only temporary directories, an existing launcher, and a writer exiting 42. Observed function status 0 and empty launcher. Add permanent failure-atomic publication regressions. |
| T1363 | Failed compensating Wi-Fi disconnect drops the route lease while the route remains active. | The isolated Rust probe calls compiled production `wifi::reconnect` with a fake Commands adapter and a changed persisted address. Successful rollback leaves connected=false and leases=0. Failed rollback leaves connected=true and leases=0, after one disconnect attempt. Retain cleanup ownership and test eventual recovery before reuse. |
| T1364 | Connect effect followed by acknowledgement failure releases the address without settling the effect. | The same probe sets connected=true before returning a connect error. Production reconnect returns None with connected=true, leases=0 and zero disconnect attempts. This synthetic counterexample establishes the policy gap; it does not measure real ADB server behavior. Add creator-settlement tests, depending on T1363 cleanup handling. |

[Evidence directory](artifacts/2026-11-12-followup-review/) contains source hashes,
logs, Cargo feature inspection, the standalone Wi-Fi probe and its lockfile,
installer probe results and the selected Android test XML. The probe is supporting
review evidence, not the permanent regression required when fixing a bug.

## Reproduction and ownership

Selected tests executed:

```sh
cargo test --locked -p blent --lib t267_disabled_auth_keeps_legacy_tokenless_video_and_idle_cleanup -- --nocapture
./android/gradlew -p android --offline testDebugUnitTest --tests 'com.blent.RemoteDisplayTest.t809_surfaceFormatStopAndStaleFramesKeepOneOwnedGeneration'
```

An initial binary-target invocation with an incomplete exact test name selected
zero tests; it supplies no verification evidence. The corrected library command
above selected and passed the intended host test.

For the installer probe, read `scripts/install.sh`, remove only its final
`main "$@"` invocation, and execute those definitions with bash. After the
definitions, point SCRIPT_DIR, APP_DIR and BIN_DIR to one temporary directory.
Create a prior `blent.desktop` there and a `write-desktop-entry.sh` containing
`exit 42`; call `install_desktop_entry`. No other installer function is invoked.

To reproduce the Wi-Fi probe, copy the retained Cargo manifest, lockfile and Rust
source to an external temporary crate, update its two absolute repository paths,
and run `cargo run --offline --locked --manifest-path <probe>/Cargo.toml`.
The fake adapter creates no sockets, ADB processes or native routes. Its Arc-owned
state lives through each completed production reconnect; the RAII lease counter
records the actual lease drop. Three sequential cases own independent state.
Production thread retirement completes before inspection. Every probe function
measures at most 2 with the project's cyclomatic checker (tree-sitter 0.25.2 and
the project Rust frontend); this is not a native lifetime proof.

The documentation link scan checked 330 relative file destinations across 38
root/current Markdown files: all 330 exist. It did not validate anchors or remote
URLs. This is a link-existence count, not a count of fully reviewed documents.
