# T888 authenticated runtime supervision

Authenticated runtime supervision gates new work on current approval while still accepting exact cleanup evidence after Stop. Draft terminal-event races are fixed; hosted interoperability does not establish physical USB retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The portable runtime retains authenticated setup and one native-session runner,
with a stable facade separating new endpoint admission from exact post-stop
cleanup. Current approval and retained connection tickets gate new work. Consumed
buffer receipts and actual native-retirement evidence can still release resources
after revocation. Factory uncertainty never grants replacement admission.

Four permanent draft regressions failed before correction: foreign-connection
terminal events escaped filtering, clean runner termination stranded setup, clean
setup termination before Ready stranded the runtime, and Ended failed to initiate
cleanup while native retirement was still pending. Other tests retain real
TabletSession borrowed-buffer cleanup, stale approval/lease rejection, invalid
clocks/capacities, native stalls and constructor/stop failures. Passing a bounded
property suite is not a claim to exhaust every possible execution path.

The Rust interoperability fixture uses production HELLO/READY/mutual-TLS setup and
two resource generations. The runtime test drives remembered reconnect/revocation
through the guarded facade and waits for actual setup/transport retirement.
Physical Android USB, AndroidKeyStore and Binder-kill acceptance remain separate.

Final validation: all 382 ordinary connection tests and 14 real Rust/Android
interoperability tests pass without failures/skips. All 32 inventoried production
functions and all 34 executable JVM methods pass individual coverage (100% line
coverage for these methods). All 129 reviewed functions have cyclomatic
complexity at most 9.

Reproduce offline with cached dependencies:

```
./android/gradlew -p android --offline --no-daemon \
  -I /absolute/path/to/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest --tests 'com.blent.connection.*' :app:profileCoverage
./android/gradlew -p android --offline --no-daemon \
  -PblentTlsFixture=/tmp/blent-t815-target/debug/examples/secure_connection_fixture \
  :app:tlsInteropTest
```

T854 still supplies production service/native construction and configuration;
T855 supplies presentation and physical pairing acceptance; T857–T860 supply real
media integration. No downloads, installations, live USB changes or network changes
occurred during this item.

</details>
