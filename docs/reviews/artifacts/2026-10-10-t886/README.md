# T886 authenticated Android setup ownership

The portable setup contract isolates pairing/trust policy from the blocking
socket/USB/storage boundary. One native authority worker performs discovery,
readiness, stock mutual TLS, durable remember/revoke and authenticated receive
window negotiation. A separate closer retains cleanup ownership. Serialized
control immediately revokes admission; cancellation is not native retirement.
The containing reactor must call `advance()` independently of native progress.

HELLO certificates are bounded and structurally validated before presentation;
they grant no trust. READY uses the remaining approval budget. TLS uses the
shorter of its native-operation budget and the remaining approval budget.
Durable writes and window negotiation have separate native budgets. Late native
completion cannot extend an expired stage before the next timer tick. Successful
consented trust writes still publish durable facts after cancellation; they do
not publish a live transport. Subsequent successful deletion publishes its own
fact. Unrelated peer deletion is bounded even after the current session is ready.

Permanent regressions were run red before correcting draft ownership, timeout
and publication bugs. Retained red logs cover thread-start side effects, clock
exhaustion, early revocation, cancelled durable-save reporting, native close
failure, TLS/fsync deadlines, late native completion before a timer tick,
unrelated deletion stalls and retirement grace measured from actual stop time.
The similarly named scratch `socket-cleanup-red.log` was a passing candidate
check (Kotlin ignores self-suppression), so it is not retained as failure evidence.

Validation uses cached dependencies and private loopback sockets. The Rust
fixture uses production discovery/READY/TLS setup and framed resource sessions;
Android runs it with its stock TLS adapter. It exercises remember/restart/revoke
and two resource generations, including API27/34 provider execution. Native
method counters supplement the source inventory with constructors/getters and
compiled callbacks; no exclusions or thresholds were weakened.

Final validation: all 12 real Rust/Android interoperability tests and all
312 ordinary connection tests pass, with zero failures or
skips. All 61 inventoried production functions pass individual coverage. All 80
executable JVM methods pass, including constructors/getters and compiled
callbacks; minimum 88.89%. All 208 reviewed production/test/fixture functions
have cyclomatic complexity at most 9.

Reproduce ordinary connection tests/counters with:

```
./android/gradlew -p android --offline --no-daemon \
  -I /absolute/path/to/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest --tests 'com.blent.connection.*' :app:profileCoverage
CARGO_TARGET_DIR=/tmp/blent-t815-target cargo build --offline --locked \
  -p blent --features accessory-transport --example secure_connection_fixture
./android/gradlew -p android --offline --no-daemon \
  -PblentTlsFixture=/tmp/blent-t815-target/debug/examples/secure_connection_fixture \
  :app:tlsInteropTest
```

T887 supplies accessory selection/permission/opening; T854 supplies production
worker composition and the explicit current-approval gate before reactor and
endpoint dispatch. Pairing presentation/physical acceptance remains T855;
production media remains T857–T860. Loopback/Robolectric do not establish physical
USB, native AndroidKeyStore or kernel Binder-kill acceptance. No downloads,
installations, USB-mode changes or network changes occurred.
