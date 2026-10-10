# T906 Android local endpoint/resource ownership

600 offline connection tests pass. All 80 scoped inventory functions and all 100
executable raw JVM methods (including constructors/getters/callbacks) meet the
individual 80% executable-line threshold. Production and permanent test functions
meet cyclomatic complexity <=9; exact entries are in `complexity.json`.

Production sources and test hashes, global source inventory, native JaCoCo XML
and execution counters, test result XML and build logs are retained alongside this
file. `AndroidConnectionWorker` is included in the complete scoped measurement.

The normal Gradle suite permanently retains T906 tests for real loopback bind,
accept and two-way traffic through production TabletSession/mux/local pumps;
control/video grouped retirement; exact stale lease rejection; partial factory
ownership; bounded event draining; queue-capacity-sized blocks; and native
startup, failed/late close and stop races. Seeded invalid native ports and lease
counters include negative and range-boundary values.

Red-before-green evidence:

- `blent-t906-native-red.log.gz`: three draft ownership regressions failed before
  retaining in-flight bind, uncertain I/O factory and failed I/O close ownership.
- `blent-t906-native-green.log.gz`: those same tests passed after correction.
- `blent-t906-late-stop-red.log.gz`: failed stop of late-created I/O falsely
  retired the backend. The identical test passed in `late-stop-green` and the
  final full coverage run after retaining late-close failure.
- `blent-t906-route-red.log.gz`: invalid lease counters incorrectly constructed
  a local route. The identical permanent bounded test passes in the final run.

Reproduce without downloads:

```sh
./android/gradlew -p android --offline --no-daemon \
  -I /home/netto/Desktop/projects/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest --tests 'com.blent.connection.*' :app:profileCoverage
```

The production media capability mask stays zero pending T907/T908. Tests establish
JVM/native-loopback ownership and protocol behavior, not physical USB, Android
Binder/Activity binding or rendered frames/touch acceptance. T855 tablet trial is
owner-deferred; T857 retains production display/input acceptance. No app was
installed, USB mode changed, or runtime token read for this implementation.
