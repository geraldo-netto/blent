# Wide-review evidence

Source revision and finding-source SHA256 hashes are in `source-manifest.json`.
`verification.json` identifies completed and interrupted work separately.
Compressed logs retain original output; embedded subprocess results are not
counted again as independent top-level Rust cases.

`ReviewRecreationProbe.kt` is review evidence, not a normal-suite regression.
It was compiled against the existing production Android runtime and unit-test
fixture classes, then run with `org.junit.runner.JUnitCore` from `android/app`.
The classpath came from Gradle's `testDebugUnitTest.classpath`; Kotlin 1.9.20
compiled with JVM target 17, no implicit standard library/reflection, and the
project classpath entries as friend paths. The unused test-parameter-injector
dependency was excluded from the compiler classpath because its newer metadata
is incompatible with that compiler; it remained on the execution classpath.
The Java invocation used `-Xmx2048m` and
`--add-opens=java.base/java.io=ALL-UNNAMED`. Compilation succeeded; JUnit exited
1 for the two expected reproductions, while four controls passed.

The probe owns its two successive Activity controllers and uses the existing
fixture's worker completion/cleanup scope. Controllers are destroyed in finally
blocks; media owners are reset and the main looper is drained. The original
worker release is completed before restoration. The fixture settles remaining
workers at teardown. No physical device or external mapping is acquired. These
finite hosted observations do not prove unrestricted native resource safety.

`media-retirement-repeat.json` records 20 isolated executions of the unchanged
compiled T991 regression. The initial failing workspace output is retained in
`rust-tests.log.gz`. The later `rust-complete.log.gz` is explicitly interrupted
at the owner's request and must not be reported as a complete pass.

`checker-scope.json` records actual complexity source-selection results;
`javascript-coverage-scope.json` records the actual coverage inventory omission.
These are source inventory observations, not native execution or metric scores.
`format.log` retains the required formatter's rejected differences without
applying them. No permanent regression was added and no defect was fixed.
