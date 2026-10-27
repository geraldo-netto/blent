# T1164 — unqualified adaptive launcher resource

The adaptive launcher resource moved without changing its XML bytes. A stale incremental APK failed the permanent packaged-resource test; regenerated outputs passed, without claiming a general build-system fix.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Move the byte-identical adaptive XML from mipmap-anydpi-v26 to mipmap-anydpi
for minSDK27. References and vector/color geometry stay unchanged. No production
function, mutable state or resource ownership changes. Static XML is bounded by
the Android resource loader; it owns the inflated drawable, with no app-owned
native handle or cleanup sponsor added.

The first incremental APK silently omitted the adaptive entry even though
merger.xml saw its new path. The permanent LauncherIconTest failed unchanged on
both SDK27 and SDK34 with “packaged launcher lost its adaptive resource”. A
forced regeneration of the exclusively owned merge/package resource outputs
restored the asset; the exact sealed test then passed both SDKs. This establishes
a stale-output issue for this relocation, not a general AGP fix. The normal-suite
test remains to reject future raster-only artifacts. No deadline or assertion
was weakened. The old and moved XML hashes match.

Final offline APK assembly and packaged resource tests pass (4 cases, zero
failures/skips);13 existing artifact/architecture Python tests also pass. aapt2's
compiled APK tree contains both original adaptive layers. Its normalized archive
path is mipmap-anydpi-v21, despite the unqualified source directory. Lint has no
errors and drops the obsolete v26 qualification advisory; themed-icon work is
separate T1166. Cached Kotlin PSI1.9.20 measures the new test at complexity1;
native Sonar remains T1019. No physical launcher, download or push was used.

</details>
