# T845: notices travel with the standalone APK

Every Android variant packages eight readable notice assets under
`assets/licenses/`. The project MIT notice and contribution attribution are exact
copies of repository LICENSE and COPYRIGHT.md. A normal Python regression detects
drift between those canonical files and the packaged source assets. Existing
copyright ownership and permission terms are unchanged.

Offline dependency review covers the release runtime graph's 85 coordinates and
59 cached binary artifacts; metadata-only modules are marked by empty artifact
lists. Cached POMs identify Apache-2.0, including ListenableFuture's inherited
Guava-parent license. Artifact hashes and license provenance are retained in the
APK inventory. JARs and runtime JARs nested inside AARs were inspected for extra
license/notice files. OkHttp's public-suffix NOTICE remains at its original path
and is also included beside the MPL-2.0 text and source location. Apache-2.0 and
MPL-2.0 texts came from the installed system's common-license files; no download
was made. Review this inventory when dependency versions or artifacts change.

The notices are APK assets, not an added screen or runtime service. APK recipients
can read them as ZIP entries without running the app. No host dependencies are
claimed to be bundled in the Android application; its separate third-party index
clarifies the cross-project references in COPYRIGHT.md.

## Permanent regression and artifact validation

Before adding assets, `LicenseAssetsTest` failed all four API-27/API-34 cases with
missing asset errors. The unchanged tests pass after packaging project notices,
complete license texts and public-suffix source notice. The tests remain in the
normal Android suite. Python notice tests additionally enforce canonical project
bytes and dependency inventory identity/provenance structure.

Offline `testDebugUnitTest --tests com.blent.LicenseAssetsTest assembleRelease`
passes. The rebuilt release-variant APK contains all eight assets byte-for-byte,
preserves OkHttp's original NOTICE, and its newly generated SDK dependency report
matches all 85 inventoried coordinates. This local artifact is unsigned because
no release-signing properties were supplied; it is packaging evidence, not an
official signed release. Nothing was installed or published. No maintained
production function changed, so no new function counters are required.

[Evidence](artifacts/2026-10-10-t845/artifact.json) records the APK and asset hashes;
adjacent compressed logs retain the failing regression, passing run and Python
tests. This resolves the APK notice finding, not a general trademark, patent,
jurisdiction-specific legal or public-release audit.
