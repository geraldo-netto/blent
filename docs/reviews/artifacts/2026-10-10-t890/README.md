# T890 configured connection shutdown budgets

Android shutdown budgets now account for configured native retirement before requesting process exit. Ended can accelerate EXIT but cannot substitute for Binder death or outstanding-call completion.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The main-process client previously sent EXIT after a fixed 500 ms, even when the
worker had a longer configured native retirement budget. Its fixed 5000 ms outer
wait could also expire before a configured native deadline. A live-Binder regression
failed on both API 27 and 34 before the correction.

Defaults now allow configured native retirement plus 1000 ms (maximum supported
actor timer cadence), followed by a 5000 ms process-exit allowance. Explicit
existing overrides retain their original bounds. A generation-matched Ended event
permits prompt EXIT, including after STOP; it never substitutes for actual Binder
death or completed IPC calls. Failed retirement retains ownership for later proof.

42 lifecycle tests pass on Robolectric API 27 and 34. All 35 inventoried production
functions and all 40 executable JVM methods have 100% line coverage. All 44 reviewed
functions have cyclomatic complexity at most 9. Permanent coverage includes early
execution completion, stuck live Binder after Ended, supported native limits,
invalid explicit limits, stale generations and existing partial-start/cleanup cases.
The combined deadline log also contains separately committed T889 regressions.

Reproduce offline with cached dependencies:

```
./android/gradlew -p android --offline --no-daemon \
  -I /absolute/path/to/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest \
  --tests 'com.blent.connection.AndroidConnectionWorkerRunTest' \
  :app:profileCoverage
```

These are framework-model tests, not physical process-kill acceptance. T855/T846
retain native validation. No installation, USB mode or network changes occurred.

</details>
