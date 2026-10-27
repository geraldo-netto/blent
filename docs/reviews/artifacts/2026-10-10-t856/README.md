# T856 portable peer and local route contracts

Portable peer identities and local route contracts distinguish native connections from ADB adapters. Collision and stale-route tests passed; contracts alone do not establish working direct-USB media.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Host attachment ownership consumes explicit peer/profile metadata and portable
link classification. ADB prefix parsing remains in its adapter and preserves old
profile keys; authenticated native peers use distinct certificate identities.
Local port claims reject collisions and retain admission through cancellation
until actual listener/bridge retirement. Configurable capacity is bounded and
lookup uses ordered maps. No socket binding or authenticated IPC is implemented
by this domain registry.

Android control/video attempts snapshot immutable local route bundles; private
worker IPC carries their revision, credential and paired local ports. Explicit
replacement invalidates stale callbacks; null routes cannot silently restore ADB.
Existing ADB defaults live only in their adapter. The permanent T864 credential
regressions remain unchanged apart from referencing the relocated ADB URL adapter.

Nine T856 Rust tests pass (four common, one counter-exhaustion unit test, four
host), alongside the 221-test common library suite, four capture contract tests
and twelve session failure tests. Android passes 74 focused API 27/34 cases and
compiles the profile variant. The Linux host binary checks offline. Tests include
4,096 port mutations, 2,048 Android endpoint mutations, malformed private IPC
numbers, counter exhaustion, collision rollback, route replacement and stale I/O.

The normal gate passes all 32 Rust functions in the route/attachment scope and
20 Kotlin functions in DisplayRoutes, VideoTransport and MediaTaskJson. For other
changed Android files, 43 affected native method/callback counters meet 80%:
VideoReceiver.connectAndReceive is 90%; the remaining counters are 100%.
Authored constructor/property counters supplement the inventory's T848 gap.
Production complexity is at most nine. Checking all 467 functions in changed
files also found one pre-existing test at ten: the T809 failed-operation case in
MediaWorkerServiceTest. T866 tracks its refactor; no full complexity pass is claimed.
Native counters, manifests, source hashes
and build/test logs are retained here; no full-project coverage claim is made.

Production broker listeners and exclusive native/ADB route selection remain
T853/T854/T857. Interface readiness does not establish ADB-free display or native
Windows/older-Android support. No runtime install, restart or download occurred.

</details>
