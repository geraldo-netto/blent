# T909 private display lease publication

Private display-route publication carries exact lease metadata without media bytes in Binder. Cancellation revokes the route before cleanup; stale closures cannot clear a replacement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

617 offline connection tests pass. All 157 scoped inventory functions and 231
executable raw JVM methods meet individual 80% line coverage. All 261 reviewed
production/test functions meet cyclomatic complexity <=9. Source/test hashes,
source inventory, native JaCoCo counters/XML and test/build results are retained.

The worker publishes an admitted route only from its guarded runtime snapshot.
An exact lease closure is distinct from actual native retirement. Private Binder
metadata carries bounded credentials/local ports; no encoded media crosses Binder.
Presentation also checks authenticated peer ticket, worker-generation reference
and monotonically increasing local resource owner. Late closures cannot clear a
replacement, and revoked routes cannot be replayed. Production capabilities remain
zero until T907 Activity composition; this item does not enable rendered display.

Permanent tests cover byte-exact IPC round trips, every truncated route, trailing
bytes, invalid ports, 128 seeded invalid counter cases, upper counter boundaries,
foreign tickets, absent media authority, duplicate/replayed route metadata, exact
stale closures, replacement process generations, actual Android Parcel/Binder
adapters under API 27/34 fixtures, worker publication, cancellation and retained
native cleanup. This is Robolectric/JVM evidence, not physical-device acceptance.

Red-before-green evidence:

- `blent-t909-cancel-red.log.gz`: main-process admission cancellation failed to
  publish display revocation before native cleanup. The same test passes in
  `cancel-green` and final coverage after synchronous exact-lease invalidation.
- `blent-t909-origin-red.log.gz`: setup metadata incorrectly forwarded new media
  event variants without runtime admission. The same test passes in final
  coverage after explicitly excluding media events from the setup event source.

Run locally without downloads:

```sh
./android/gradlew -p android --offline --no-daemon \
  -I /home/netto/Desktop/projects/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest --tests 'com.blent.connection.*' :app:profileCoverage
```

T907 owns foreground/background, Activity recreation and SessionCoordinator
composition. T908 owns host registration; T857 owns joined rendering/input
acceptance. T855 physical pairing remains owner-deferred. No installation or
USB mode change occurred.

</details>
