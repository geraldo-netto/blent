# T992: Linux leased speaker PCM composition

Linux uses an independent typed Connection/Invitation port with focused ADB and
USB boundary adapters. USB speaker selection is explicit, preserves the original
full peer/profile/credential and never falls back to ADB. USB microphone remains
unavailable until T859. Pending consent and actual streaming monitor original
connection loss. The existing authenticated bootstrap negotiates tablet processing
and optional native clocks; buffering/background and stereo PCM remain unchanged.

The permanent pre-stopped regression failed before the fix: native dependency
lookup ran after Stop. The same test passes without native setup. Existing ADB
microphone/speaker pipeline, signals and exact mapping cleanup regressions remain.
The unavailable-direction regression now covers both directions in default builds
and microphone in accessory builds; USB speakers have separate implemented-path
coverage. Bounded malformed credential/profile tests reject before native work.

Private fixtures join real mutual TLS, broker FD registration, management/mux,
loopback PCM and synthetic native helpers. Assertions cover exact stereo samples,
processing override, clock fields, pending Stop/loss, original source withdrawal,
failed/stalled helpers, blocked actual dependent leases and delayed remote consumer
retirement. This join exposed the independently fixed T994 queued cancellation
race. A pending actual completion remains owned after bounded errors; T995 tracks
the remaining controller admission follow-up. No acoustic/device qualification.

Fresh offline accessory collection: 37 host + 49 common tests pass; default:
34 host + 49 common tests pass. All 28 affected production functions meet individual
80% executable-line coverage, without missing counters. Changed Rust complexity:
92 functions, maximum nine. Both collections use identical immutable build inputs
and separate exclusively owned native targets. Compressed manifests, attestations,
LCOV, source/native inventories, red/green logs and counters retain the evidence.
GUI selection remains T979; physical speaker acceptance T860, SDK-wide native
audio qualification T835 and Windows native validation T902 remain explicit.
No downloads, installation or live host/tablet trials.

The attested working tree includes the pre-existing uncommitted TLS fixture.
Those unrelated edits remain unstaged and are not included in this commit.
