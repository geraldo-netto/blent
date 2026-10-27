# T987: completed local producer scratch reuse

Completed partial-read scratch now reuses one array only after exact completion and transport acceptance. Full transferred blocks remain independently owned; local JVM allocation savings do not establish Android power gains.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`LocalChannelProducer` retains at most one completed scratch array for partial
or empty reads. A new native request may borrow it only after the exact original
read completes and any copied pending payload is accepted by the transport.
Each request remains a fresh identity. Full reads transfer their original array
permanently, so subsequent reads allocate independent storage. Neither timeout,
cancellation nor logical stop makes an outstanding read reusable or proves
native retirement. Transport-owned arrays are never mutated or recycled.

Two permanent normal-suite regressions fail before the fix: completed partial
scratch is discarded after acceptance, and empty completions allocate again.
Both pass afterward. Additional tests cover every partial extent of an eight-byte
block, minimum/maximum block boundaries, immutable full and partial transferred
bytes, backpressure, stale completions sharing the reused array, invalid counts
after reuse, stopped/stuck original ownership, and independent sibling pumps.
Existing pump/lifecycle tests remain unchanged. The JVM allocation observer is
shared with the retained T990 measurement, preserving its assertions.

The matched uninstrumented OpenJDK 21.0.12.1 fixture uses a 65,536-byte native
read block, 64 warmup turns and three samples of 256 turns per case. Microphone
partial reads contain 1,920 bytes; display partial reads contain 32,768. These
are synthetic channel extents, not physical audio/video delivery measurements.

Median allocated bytes per turn:

| Case | Before | After |
| --- | ---: | ---: |
| Held idle native read | 0.1875 | 0.1875 |
| Empty completed read | 65,616.1875 | 64.1875 |
| Partial microphone-channel read | 67,576.1875 | 2,024.1875 |
| Full microphone-channel read | 65,640.1875 | 65,640.1875 |
| Partial display-channel read | 98,424.1875 | 32,872.1875 |
| Full display-channel read | 65,640.1875 | 65,640.1875 |

Partial/empty completion saves exactly one 65,552-byte array allocation in this
fixture. Full blocks retain their ownership-required allocation; held idle reads
already allocated nothing per turn. Fractional overhead includes the shared JVM
observer. Raw allocation and elapsed-time samples remain in `benchmark.json.gz`;
timing is observational and never a test threshold. No Android CPU, power or
complete-path latency gain is claimed.

Full offline debug JVM/Robolectric coverage passes 1,972 tests, with zero failures,
errors or skips, including all eight T987 tests and retained T905/T990 regressions.
The existing separate native TLS integration task is unchanged. All 33 maintained
production methods across `ConnectionLocalPump.kt` and `ConnectionMediaAccess.kt`
exceed 80% native JaCoCo line coverage (minimum 93.75%); both changed producer
methods reach 100%. Source fingerprints, native XML/raw execution counters,
compiled-class hashes and per-method results are retained. No tests, assertions
or authored-method counters were removed, skipped or weakened.

The cached Kotlin 1.9.20 PSI source audit measures 92 changed-file methods,
including test fixtures, with maximum cyclomatic complexity 6. This independent
Sonar-rule audit does not replace native SonarQube measurement; T1019 retains
that unavailable evidence. JaCoCo 0.8.15 and Gradle 8.5 use existing cached
dependencies and Robolectric jars. Four architecture-boundary tests also pass.
No downloads, installs, uploads, service changes or device tests occurred.

```sh
./android/gradlew -p android --offline --no-daemon \
  -I /home/netto/Desktop/projects/blent/scripts/coverage/android.init.gradle \
  -I /tmp/blent-t990-evidence/offline.gradle :app:profileCoverage
python3 /tmp/blent-t987-evidence/check_kotlin.py
python3 -m unittest scripts.tests.test_android_contract_boundaries \
  scripts.tests.test_android_lifecycle_boundaries
```

The reused offline initialization script is retained in T990's evidence directory.

</details>
