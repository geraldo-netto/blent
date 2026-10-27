# T891 USB fault-fixture isolation

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The T887 native request-failure test left its injected exception in shared shadow
state. Later production-entry tests failed with that old exception on both Android
API models. A permanent regression directly invokes the native failure case and
checks that its injection does not escape; it failed on API 27 and 34 before repair.

The fixture now restores fault state and closes its permission owner in finally.
Existing native-failure and cancellation assertions remain unchanged. Both new
regressions pass, and the combined working-tree connection suite passes all 516
tests with no failures/skips (including pending T854 composition). No production
behavior or coverage exclusions changed. Retained logs distinguish focused and
combined validation; the latter includes T854 work committed separately.

Reproduce the permanent fixture regression offline:

```
./android/gradlew -p android --offline --no-daemon :app:testDebugUnitTest \
  --tests 'com.blent.connection.AccessoryFixtureIsolationTest'
```
