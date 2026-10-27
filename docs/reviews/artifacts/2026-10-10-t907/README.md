# T907 Android display route and Activity composition

Android display routes now require authenticated matching peer authority and preserve camera/audio independence. Hosted Activity tests and mask-1 negotiation establish composition, not physical rendered frames or touch.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

865 selected connection, ADB, UI/settings and lifecycle tests pass offline.
All 124 scoped functions and all 204 executable JVM methods (including authored
constructors/getters/callbacks) meet individual 80% executable-line coverage.
All reviewed production and permanent test functions meet cyclomatic complexity
<=9; see exact entries in `complexity.json`. Native JaCoCo execution/XML, inventory,
source/test hashes and build/test results are retained alongside this document.

Android advertises only display capability. Independent DisplayEndpoint and
DisplayRouteTarget contracts separate the connection adapter from session and OS
implementations. Session-local revisions span USB and explicit subsequent ADB
selection, never wrap, and never advance on invalid destinations. Control/video
are revoked together before replacement. Camera/audio owners remain independent.

Permanent normal-suite tests cover API 27/34 Activity start/stop/recreation,
background route loss, connection-owner retention, absence of an admitted route,
stale control callbacks, backend changes, saved versus freshly delivered legacy
credentials, partial installation failure and revision exhaustion. Bounded tests
cover invalid ports, credential strings, counters and 256 seeded byte-exact
credential conversions including signed-byte edges.

`blent-t907-snapshot-red.log.gz` reproduces an Activity adapter granting a route
from a malformed unauthenticated snapshot. The identical regression passes in
final coverage after requiring active authenticated presentation and matching
peer ticket. Existing valid fixture snapshots were supplied their required peer
metadata; the failing invalid snapshots were retained unchanged.

`blent-t907-capability-red.log.gz` is the feature acceptance test against the
previous deliberately disabled capability mask. Final tests verify the production
worker emits mask 1 after real management-credit exchange; camera/audio stay off.
This was a planned feature transition, not evidence that the old mask was a bug.

Final coverage ran `com.blent.connection.*`, SessionDisplayRouteTest and the
existing SessionRevocation, TokenDelivery, SessionSurfaceContract, SettingsLayout,
StreamFormat, ReleaseCheck, StreamingPower, ActivityUiContract, Regression,
DisplaySettings, SettingsControlContract, SettingsAccessibility, Orientation,
DecoderDiagnosticsUi, DecoderWaitUi, CameraControls, AudioControls,
SurfaceCallbackContract, CalibrationStartup and DisplayRoutes test classes using
`--offline --no-daemon`, `scripts/coverage/android.init.gradle` and
`:app:profileCoverage`.

These are JVM/Robolectric and native bytecode counters. Physical USB/rendered-frame
and touch/mouse acceptance remains T857; host session registration remains T908.
T855 physical pairing is owner-deferred. No installation, reload, external download
or USB mode change occurred. Existing installed software was not modified.

</details>
