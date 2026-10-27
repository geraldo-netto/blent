# T1076: Activity teardown and pairing UI sandbox isolation

Ordered tests reproduced pairing UI failures after Activity teardown in a shared Compose sandbox. Separating fixture sandboxes restored all 118 focused cases without weakening assertions; production behavior did not change.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The focused normal runner reproduced eight pairing UI idle failures on SDK27/34
before their test bodies; 108 other cases passed. The unchanged complete T1061
instrumented suite had passed them. Orientation's real MainActivity teardown and
ordinary pairing setContent tests shared Compose platform instrumentation. The
permanent PairingUiOrderTest explicitly runs the native Activity orientation case
then the original pairing dismissal assertions on each SDK, requiring one actual
execution of each. Both ordered regressions fail before the fixture change.

Pairing now uses the ordinary UI sandbox; Orientation retains its existing separate
Compose platform instrumentation. Both exact ordered regressions pass afterward.
No dispatcher internals are reset, no assertion or exception is suppressed and
the existing 60-second Espresso deadline remains unchanged. The evidence isolates
this shared sandbox lifecycle condition; it does not claim a particular private
Compose field as the proven cause. There is no production application change.

Final clean offline focused collection: **118 tests, no failures/errors/skips**;
all unit sources compile without warnings. It includes T1072 warning cleanup,
which has its own commit/evidence. All source hashes match the frozen inputs.
Kotlin PSI1.9.20 source audit covers 32 test functions/callbacks, maximum
estimate 5; native SonarQube remains T1019. No production function changed.

Original JVM stacks and bounded read-only queue observations (at most16 messages
per loaded Looper) retain the diagnostic uncertainty. The first run progressed
through all eight failures and exited normally; it was never cut off for elapsed
time. Its owned test process had no children. The diagnostic agent retained no
callback, altered no queue and ended with that test JVM; probe sources are retained.
Test workers/Activities remain owned by their original fixture and Robolectric
lifecycle. Ordered and final results preserve normal cleanup. No hardware, native
process-fence, physical power, network or unrestricted safety claim is made.

</details>
