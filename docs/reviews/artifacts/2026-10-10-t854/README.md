# T854 production USB composition

Android production USB composition joins permission, isolated setup and session ownership without sending media through Binder. Component tests passed with media capability still disabled pending endpoint integration.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The main-process entry now links supervised USB permission admission to the private
connection process. The service factory lazily composes USB opening, retained
socket bridge, Android identity/trust, authenticated setup and the session runner.
Native work stays off Binder/UI/control callbacks. Configuration forwards validated
capacities and deadlines; no media travels through Binder. Media capability is
zero until T857 supplies real endpoint ownership.

Permanent red-to-green regressions cover cancelled execution with a nonadvancing
clock, clean permission retirement before Ready, an uncooperative child outliving
cancellation, and accidentally resolving coroutine-scope cancellation from inside
NonCancellable cleanup. Tests retain native factory/open/close uncertainty, late
acquisition, permission-before-binding, detach with stale pairing callbacks,
actual Binder-death retirement, failure isolation and bounded malformed input.
The initial worker log also contains a corrected exception-identity fixture
assertion; the cancellation regression itself remained unchanged. The composition
log captures the three confirmed lifecycle causes and their original integrations.

Production helper metadata matches the Android accessory filter. The explicit
prototype helper route retains its own profile. Metadata selects an application;
it does not authenticate a computer. Directory tests preserve invalid or existing
state. Synthetic test-only KeyStore material exercises loader/signature wiring;
it is not evidence of real hardware key isolation.

Final verification, all offline with cached dependencies:

- 518 ordinary connection tests, no failures/errors/skips.
- 14 real Rust/Android HELLO/READY/TLS interoperability tests, no failures/skips.
- 22 native-helper tests using ASan/UBSan fixtures.
- All 119 inventoried Kotlin functions and all 159 executable JVM methods meet
  individual 80% line coverage (minimum JVM method: 80%).
- All 18 C functions meet individual coverage; minimum is 12/13 lines (92.31%).
- All 283 reviewed production/test functions have cyclomatic complexity <=9.

Reproduce using the cached local Rust fixture and installed compilers:

```
./android/gradlew -p android --offline --no-daemon \
  -I /absolute/path/to/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest --tests 'com.blent.connection.*' :app:profileCoverage
./android/gradlew -p android --offline --no-daemon \
  -PblentTlsFixture=/tmp/blent-t815-target/debug/examples/secure_connection_fixture \
  :app:tlsInteropTest
python3 -m unittest scripts.tests.test_aoa_probe \
  scripts.tests.test_accessory_selection scripts.tests.test_accessory_discovery
```

Native C counters additionally compile the permanent enumeration/bulk fixtures
with `-O0 -g --coverage -fprofile-abs-path`, execute their existing error/selection
cases and export `gcov --json-format`. Exact source hashes, raw counter XML/exec,
gcov JSON, complete unit-test results and failure/pass logs are retained here.

T855 still wires presentation/activity ownership and qualifies production pairing
on the tablet. T857–T860 retain real media routing/acceptance; T862/T846 retain
physical combined lifecycle acceptance. No ADB-free production-media support is
claimed by these component/interoperability tests. No downloads, installations,
live USB switches or network changes occurred.

</details>
