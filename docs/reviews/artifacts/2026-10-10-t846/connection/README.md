# T846 connection foundation evidence

Offline local validation, October 10, 2026. Source hashes identify the unfinished
integration under review. This is not installed production USB support.

- `rust-tests.log.gz`: 37 selected Rust tests pass: domain, TLS and owned setup.
- `android-tests.log.gz`: 71 JVM/Robolectric connection and synthetic completion tests pass.
- `timeout-red.log.gz`, `credit-red.log.gz`, `descriptor-rollback-red.log.gz`:
  permanent regressions reproduced leaked handshake timeout, oversized scheduler
  credit acceptance, and descriptor rollback leakage. `finish-red.log.gz` and
  `approval-red.log.gz` additionally reproduce premature synthetic success and
  blocked console timeout shutdown; current Android and approval-green logs pass. Unchanged tests pass in the
  current logs. `credit-green.log.gz` retains the original focused green run.
- `coverage.json.gz`: all 79 inventoried Rust and 96 Kotlin functions meet the
  per-function 80% threshold; LLVM/JaCoCo inputs and source manifest retained.
- `constructors.json`: supplementary native counters for 22 Kotlin constructors
  omitted by existing inventory (T848); all 100%. Compiler default-argument
  constructors are excluded from this supplementary list, not authored bodies.
- `native-c-coverage.json.gz`, `native-c-tests.log.gz`: 26/26 native accessory
  functions meet 80%; ten tests pass. The normal driver additionally exercises
  these cases under ASan/UBSan; gcov measurement itself is a separate build.
- `ready-{red,green}.log.gz`, `key-digest-{red,green}.log.gz`,
  `nodelay-{red,green}.log.gz`, `zlp-write-{red,green}.log.gz`, and
  `diagnostics-{red,green}.log.gz`: permanent regressions for human approval
  skew, stock native TLS digest authorization, delayed small socket writes,
  packet-aligned USB output completion and preservation of the primary error.
- `secure-attempt6-adb.log.gz`, `secure-attempt6-{progress,result}.txt.gz`:
  Android 16 native encrypted diagnostic pass: 4,096 echoes, 58,803,628 payload
  bytes each direction in 16.158 seconds; final receipt, TLS close notification
  and actual native bridge retirement confirmed. ADB was enabled (`18d1:2d01`).
  Earlier attempt 3–5 journals preserve the native failures that informed fixes.
  `old-key-rejected-progress.txt.gz` proves incompatible persisted keys reject
  before opening USB. Only the disposable profile app was cleared to regenerate
  its test key; production identity was untouched.
- `secure-attempt7-debugging-off.log.gz`, `secure-attempt7-descriptors.json.gz`
  and `secure-attempt7-summary.json.gz`: encrypted accessory-only host pass,
  4,096 echoes in 16.498 seconds, same payload totals, USB `18d1:2d00` with one
  interface and no ADB. `secure-attempt7-{progress,result}.txt.gz` independently
  confirm host receipt and actual native bridge retirement on the tablet. This remains synthetic transport qualification only. Debugging and normal host
  streaming were then restored; `secure-attempt7-restored-stream.log.gz` records
  resumed render acknowledgements. `native-trial-sources.json.gz` identifies the
  tested source; subsequent profile/host prompt wording explicitly distinguishes
  corresponding labels from the two different identities. Transport code is unchanged.
- `coverage-tools.log.gz`: 71 coverage-tool regressions pass after native scope
  expansion.
- `complexity.json.gz`: 220 production functions in the five scoped source directories, maximum cyclomatic complexity 9.
- `tls-interop.log.gz`: normal offline Rust/Android JVM interoperability driver.

Rust measurement includes `--lib --test connection --test connection_mux
--test connection_pairing -- t846_` with `cargo llvm-cov --offline --locked
-p blent-config -p blent --features accessory-transport --lcov`,
`CARGO_NET_OFFLINE=true` and a separate coverage target directory.

Android measurement uses `./android/gradlew -p android --offline --console=plain
-I "$PWD/scripts/coverage/android.init.gradle" :app:testDebugUnitTest
--tests 'com.blent.connection.*' --tests 'com.blent.AoaSecureEchoTest'
--tests 'com.blent.AoaSecureDiagnosticsTest' :app:profileCoverage`.

The retained source manifest, LLVM and JaCoCo data pass
`scripts/coverage/report.py check` scoped to `common/src/connection/`,
`host/src/secure_connection/`, `host/src/accessory/`, and
`android/app/src/main/java/com/blent/connection/`.
Native C uses `scripts/coverage/capture_check.py OUTPUT --scope host/accessory/`.

Rust/Android JVM TLS interoperability includes API 27 and 34 branches. Native
Android TLS is qualified separately on Android 16 by attempts 6 and 7. API 27/28
native qualification, production trust/UI, broker
wiring and physical media acceptance remain open. No whole-project pass is implied.
