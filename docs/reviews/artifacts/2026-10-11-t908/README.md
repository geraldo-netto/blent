# T908 Linux production USB display composition

The Linux daemon now exports its actual primary listeners and composes T924/T925/
T927 discovery, capacity and joined ownership when built with `accessory-transport`.
The default build retains ADB behavior. Native launch/path/IPC and TCP capabilities
remain in Linux adapters; shared discovery, reservation, authority and retirement
contracts remain independent of capture, storage and UI implementations.

Authenticated selected peers reserve capacity before independently progressing
startup jobs. Primary assignments reuse the daemon runtime and listeners; extras
prepare the same shared session specification on demand. A separate 100 ms native
timer observes admission and retirement without increasing the existing two-second
ADB inventory cadence. Selected USB uses a free slot first, otherwise retires one
ADB display and waits for its existing cleanup proof. Ambiguous new peers cannot
preempt it. Status publishes only exact Active registration with healthy work.

Cancellation, source loss and late startup retain ownership. Shutdown closes
registrations promptly and joins pending startup/actual session retirement. Retained
listener capabilities are destroyed before capacity is reused. A startup task panic
cannot establish native cleanup proof and therefore quarantines its ticket while
other peers continue; independent recovery proof remains explicitly tracked T928.
Optional-ADB diagnostics remain separately tracked T929; idle discovery allocation
evaluation remains T930. No EVDI or external dependency code changed.

The permanent primary-handoff regression failed before its fix: the draft accepted
unrelated listener capabilities. It now validates the configured ports and actual
primary runtime before discovery. Ten native daemon tests cover retained primary/
extra ports, bind rollback, invalid instances, ambiguity, ADB preemption, typed
status publication, timer separation, stalled/panicked startup, late cancellation
and pending-startup shutdown.

The joined protocol fixture uses real mutual TLS, broker discovery/registration,
exported TCP listeners, the actual stream/WebSocket servers and joined ownership.
It sends fresh registration credentials over both channels, receives configuration
and sequence-bearing synthetic frame packets, sends a frame ACK and verifies touch
dispatch after it. Geometry/capture gates activate, and disconnect retires routes,
native session work and listener ports. Synthetic capture/input avoid devices;
this is not codec decoding or physical Android rendered-frame acceptance. T857
retains joined Android/physical display and touch/mouse acceptance, including the
postponed T855 production pairing trial. Camera integration remains T858.

Validation passes: 639 host library tests (three existing benchmark ignores),
435 native-feature binary tests and 425 default binary tests (four existing ignores
in each), and both real idle-daemon/diagnostic lifecycle integrations in both
builds. All 31 scoped production functions meet individual 80% executable-line
coverage; all 200 reviewed functions have cyclomatic complexity at most nine.
Counters export exact fresh native library/binary/integration mappings, including
the real daemon executable. Offline Windows production/test checking passes;
native Windows USB/runtime counters remain unsupported and tracked T902.

An initial full binary run had seven failures after another Cargo invocation
rebuilt its still-running executable. Child re-execution then returned NotFound,
and the changed executable fingerprint invalidated a cache fixture. The same full
suite passes with builds serialized; no assertions were skipped or weakened. Both
the failure and passing logs are retained. Source hashes and inventory correspond
to the final implementation.

```sh
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent --features accessory-transport --lib
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --bin blent --lcov --output-path /tmp/blent-t908-full-bin-coverage.lcov
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --test linux_lifecycle --lcov --output-path /tmp/blent-t908-lifecycle-coverage.lcov
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent --bin blent
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent --test linux_lifecycle
CARGO_NET_OFFLINE=true cargo check --offline --locked --target x86_64-pc-windows-gnu -p blent -p blent-gui --tests
```

No downloads, installation, reloads, live broker/device changes or physical trial.
The separate postponed T855 TLS fixture remains uncommitted.
