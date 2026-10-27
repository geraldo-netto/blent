# T884 private Android connection process and IPC

The private Android connection process accepts bounded typed control IPC and retains canceled native calls. Hosted Binder lifecycle tests require death plus outstanding-call return, without claiming physical termination acceptance.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

A dedicated nonexported `:connection` process provides the ownership boundary
for future USB/TLS/native setup and retirement. Its independent typed contract carries
only bounded control metadata. Encoded records are at most 4 KiB, Parcels at most
8 KiB, and command/event queues each hold at most 32 records. Same-UID one-way
transactions require exact version, generation, type, length and trailing-data
validation. Media bytes never enter this Binder contract.

The service admits one generation and one START. The client owns asynchronous
bind/START/control operations independently of their awaiting caller. Native
calls that outlive cancellation remain owned. STOP cannot prevent its separate
EXIT admission. Ended events and framework binding invalidation do not establish
process death. Retirement requires actual Binder death and return of outstanding
local calls; a failed retirement keeps ownership quarantined.

Permanent regressions cover codec invalid inputs and bounded mutation, malformed
parcels, foreign UID, stale generation, blocked setup/cleanup, queue saturation,
callback delivery failure, single admission and actual-death requirements. Draft
callback-link failure and client/service generation mismatch were reproduced
before their fixes. Two late-binding retirement races and a failed death-link readiness race were
also reproduced before their fixes, with all regressions retained.

Offline validation uses cached dependencies:

- All 265 ordinary connection tests pass (no skips), including 36 API27/34 client
  tests and 22 service tests.
- All 85 inventoried production functions pass the per-function coverage gate.
- All 116 executable JVM methods, including constructors/getters and separately
  compiled callbacks, exceed 80% line coverage; minimum 85.71%.
- All 134 reviewed production/test functions have cyclomatic complexity at most 7.

Reproduce with `./android/gradlew -p android --offline --no-daemon
-I /absolute/path/to/blent/scripts/coverage/android.init.gradle
:app:testDebugUnitTest --tests 'com.blent.connection.*' :app:profileCoverage`.
 Native method counters include
constructors/getters omitted by the source inventory under open item T848. No
coverage exclusions or thresholds were weakened. The saved inventory precedes
the final counter run; sources.json records relevant production/test hashes.

The production worker factory explicitly remains unsupported until T854 wires
accessory permission, identity/trust and authenticated setup. T855 supplies
pairing presentation. T857 supplies local media endpoints and actual production
display/touch acceptance. Robolectric API27/34 tests model Binder lifecycle; they
do not establish physical-device USB or kernel Binder termination behavior.
No downloads, installations, USB-mode changes or network changes occurred.

</details>
