# T861: native USB distribution boundary

Linux distribution layouts require both USB helper executables and their runtime dependencies. Packaging checks and offline builds passed; missing pinned libevdi prevented complete portable-release staging.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Linux release/source builds enable accessory-transport and build the unchanged
stock-libusb C helper. Tar/AppImage staging requires both native USB executable
siblings; user installation requires the pair before publishing its staging
transaction. AppImage ABI, RPATH, loader and transitive source/notice collectors
include both; Linux runtime/build package prerequisites and Arch install/build
recipes carry them. Android uses its existing packaged stock TLS/KeyStore path;
its current debug APK builds with the runtime license inventory intact.

Original package/layout/dependency regression failures are retained before
repair, including each missing pair member. Nine permanent new normal-suite
regressions pass, plus28 AppImage,14 installer,7 Make/jobserver/dry-run,6 package,
1 Arch build, two actual-Cargo Arch cases and two desktop-path cases. Existing real-tool
fixtures now supply the current USB artifacts/features; original fresh-output,
reserved-character, rollback, pinned target and original library-mapping checks
remain. Fixture argument/stale-input failures are retained separately. No test
was skipped/weakened. All seven changed production packaging functions measure
100% individually using native Bash locations and coverage.py line events.
Unchanged repository collectors verify byte-identical extracted function origins;
raw traces/manifests and the cached ABI14 grammar adapter are retained. Local
source complexity estimates <=9; unavailable native Sonar remains T1019.

Current C helper and Rust broker build offline; actual sibling ELF dependency,
CLI and invalid-invocation smoke pass without opening USB. Existing libusb and
cached headers are used; no packages/sources were downloaded. Hashes/APK license
entries and raw native build logs retained. Full portable release staging was
not possible with the current absent pinned libevdi1.15.0; the existing1.15.1
file is not relabelled. T1209 retains that excluded native-artifact prerequisite.
Isolated package/ABI/loader fixtures establish this packaging behavior, not a
complete release qualification, signing result, hardware pairing/media result
or Android27/28 native provider support. T846/T863/T855–T860 retain those scopes.

Binary/config path ownership stays in existing Linux launch adapters; no search
fallback, old-package compatibility path or duplicate mutable owner was added.
Installer temporary-directory owner still removes rejected staging and publishes
only after all required copies/chmod succeed. AppImage builder owns its output
and stock dependency closure; user-selected build/cache paths retain artifacts
until explicit cleanup. No new native lifetime/retirement mechanism. Original
USB/runtime worker owners and device acceptance stay unchanged. No service,
physical tablet/display, EVDI rebuild/replacement, transfer or upload.

</details>
