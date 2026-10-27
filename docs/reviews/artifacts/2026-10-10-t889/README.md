# T889 supervised USB admission

USB admission separates native-operation deadlines from human consent time. Late grants and canceled startup cannot admit replacement workers before actual cleanup.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

One portable admission owner retains a native permission worker and independent
closer. Android handles accessory inventory, permission callbacks and detach
registration behind an explicit boundary. Constructor/control methods perform no
native I/O. Detach invalidates selection synchronously; permission metadata never
grants TLS or media authority. Stale/duplicate results cannot admit another worker.

Four permanent draft regressions were demonstrated before correction: duplicate
grant publication, enumeration after a cancelled late start, and blocked native
request/callback work inheriting the human approval timeout. The native deadline
is now independent of the absolute consent deadline. Other retained tests cover
failed dispatch, bounded callback queues, native stalls, late registration,
uncertain construction, failed cleanup and actual thread/resource retirement.

45 tests pass, including real owner/Android adapter integration on Robolectric
API 27 and 34, plus seeded stale-callback mutations and invalid-input bounds.
All 48 inventoried production functions and all 52 executable JVM methods have
100% line coverage. All 103 reviewed production/test functions have cyclomatic
complexity at most 9. Raw counters, source hashes and red/green logs are retained.
The combined deadline log also contains the separately committed T890 regression.

Reproduce using cached dependencies only:

```
./android/gradlew -p android --offline --no-daemon \
  -I /absolute/path/to/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest \
  --tests 'com.blent.connection.AccessoryAdmissionOwnerTest' \
  --tests 'com.blent.connection.AndroidAccessoryAdmissionBackendTest' \
  :app:profileCoverage
```

T854 connects this admission to the private connection process; T855 supplies UI
and physical debugging-off pairing acceptance. These tests do not establish actual
USB/Android framework retirement on the tablet. No device or network changes.

</details>
