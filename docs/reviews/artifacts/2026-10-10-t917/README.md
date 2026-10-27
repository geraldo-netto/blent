# T917 shared display capacity

Shared display capacity prefers free slots and retains retiring work until exact cleanup. The recorded automatic USB-preemption policy is distinct from the later cooperative manual Start behavior.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Owner chose automatic replacement of an ADB display by explicitly selected USB.
The portable capacity owner reserves free slots first, otherwise selects one ADB
victim, preferring an extra. It never equates an ADB serial with a certificate.
Retiring native work counts against capacity until the adapter supplies exact
completion; changing selection and late completion cannot free another owner.

Existing monitor preparation and retirement use these tickets. Its primary
runtime remains daemon-owned; extra runtimes retain T919 native completion.
T908 still supplies production selected-peer observation, registration and
native-resource composition. No physical USB or rendered-frame claim here.

Permanent draft regressions failed before their fixes:
- Legacy primary reselection invalidated a native display's attachment lease.
- Exhausted ticket counters requested eviction despite being unable to admit a
  replacement. Working ADB display now survives exhaustion.

Validation: 421 host binary tests passed, with four pre-existing opt-in benchmarks
ignored; six portable slot tests passed. All 43 maintained functions in the four
production scopes meet individual 80% executable-line coverage. All 99 functions
in changed production/test scopes have cyclomatic complexity at most 9.
Bounded invalid-input tests cover capacity, index, serial length/control characters,
peer mismatch, duplicate ownership, stale tickets and counter limits. Monitor
fixtures cover primary/extra handoff, startup rollback, blocked native retirement,
reconnect and ADB behavior. The three-device token-recovery fixture now configures
three slots instead of claiming three assignments under a two-slot configuration.
No existing regression was skipped or weakened.

Counters were exported from the exact current bin/lib test objects, excluding
stale cached objects (T880). The global source inventory also includes the separate
pending T855 test; it is outside this change and these coverage scopes.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --bin blent --lcov --output-path /tmp/blent-t917-bin.lcov
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --lib --lcov --output-path /tmp/blent-t917-model.lcov -- display_slots::
llvm-cov export --format=lcov --instr-profile=target/llvm-cov-target/blent.profdata --object target/llvm-cov-target/debug/deps/blent-3d154ee040abf851 --object target/llvm-cov-target/debug/deps/blent-276264e4cf99d16f > /tmp/blent-t917-current.lcov
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t917-manifest.json --lcov /tmp/blent-t917-current.lcov --scope host/src/display_slots.rs --scope host/src/monitor.rs --scope host/src/monitor/retirement.rs --scope host/src/monitor/admission.rs --output /tmp/blent-t917-coverage.json
```

No downloads, installs, device mode changes, reloads or dependency patches.

</details>
