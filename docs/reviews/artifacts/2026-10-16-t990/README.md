# T990: immutable endpoint projection per publication

`ConnectionMediaAccess` retains one immutable publication containing the exact
choice reference, original offer and typed endpoint metadata. Unchanged sibling
or target wakes reuse that metadata; a new choice or peer reprojects it even
when its lease and endpoint values compare equal. Withdrawal clears the cache.
Every capture still selects the currently authenticated choice and matches its
exact publication and current target endpoint. Endpoint equality never grants
consent, revives stale callbacks or proves native retirement. Existing typed
camera/audio adapters and independent original-consent ownership stay in place.

The permanent `t990` normal-suite regression fails before the change: 65 captures
of one unchanged publication invoke the endpoint adapter 65 times instead of
once. The same regression passes afterward. Further tests retain equal-metadata
choice/peer replacement, stale original authority, mismatched targets, source
withdrawal and foreign leases. All existing audio and camera access tests pass.

The retained uninstrumented speaker fixture performs 128 warmup captures and
three samples of 1,000 captures on OpenJDK 21.0.12.1. Original allocation is
20,542,952–21,409,960 bytes per sample; cached allocation is 24,048–25,024 bytes.
Median allocation per capture falls from 21,409.024 to 24.232 bytes. Original
sample times are 16.16–24.24 ms, cached times 0.144–0.877 ms. These bounded local
JVM observations support avoiding repeated credential formatting and endpoint
allocation; they do not establish Android CPU, battery or end-to-end latency
gains. Timing is observational, never a flaky test threshold. Raw samples and
the before/after projection counts remain in `benchmark.json.gz` and test XML.

Full offline debug JVM/Robolectric coverage passes 1,964 tests with zero failures,
errors or skips. The existing separate native TLS integration task is unchanged.
All 14 maintained production methods in `ConnectionMediaAccess.kt`, including
constructors and separately compiled callbacks, reach 100% JaCoCo line coverage.
The existing repository inventory and native-method mapping apply their usual
inline-body attribution; no counters or authored methods are newly excluded.
Source fingerprints, compiled-class hashes, XML, raw execution counters and
per-method results are retained. Four architecture-boundary tests also pass.

The repository's cached Kotlin 1.9.20 PSI source audit covers 53 changed-file
methods, including tests, with maximum cyclomatic complexity 6. This is an
independent Sonar-rule count, not native SonarQube evidence; T1019 retains that
missing measurement. JaCoCo is 0.8.15 and Gradle 8.5. All tool dependencies were
already cached. Robolectric offline resolution uses only links to existing
local Android jars; no downloads, installs, uploads or device changes occurred.

```sh
./android/gradlew -p android --offline --no-daemon \
  -I /home/netto/Desktop/projects/blent/scripts/coverage/android.init.gradle \
  -I /tmp/blent-t990-evidence/offline.gradle :app:profileCoverage
python3 /tmp/blent-t990-evidence/check_kotlin.py
python3 -m unittest scripts.tests.test_android_contract_boundaries \
  scripts.tests.test_android_lifecycle_boundaries
```
