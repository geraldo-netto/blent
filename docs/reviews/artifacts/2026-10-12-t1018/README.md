# T1018: owned activity source following

Independent common creation, publication and lifetime contracts separate the
portable controller from Linux adapters. Following reserves capacity before
asynchronous factory exposure, applies configured activity preferences and
publishes pending/observed/failed/stopped states. Observed snapshots include
immutable capabilities and retained host stop fences; unsupported capabilities
remain explicit. Actual queue bounds belong to the source; each controller turn
consumes at most the configured capacity without misclassifying replenishment.
Identical publication does not wake consumers, and stop prevents late revival.

SourceLifetime retains the original admission until last-clone destruction and
every registered creator/task/child completion certificate. Native tasks retain
leases through actual join; helpers retain them through reap and cleanup. A
timed-out or dropped constructor can neither free a blocked creator nor abandon
a late child. The Linux factory keeps D-Bus and independently owned X11 observation.
Factory creation must return its future promptly; actual native work belongs to
owned adapters and must retain/register its original lifetime certificates.

Permanent normal-suite regressions initially fail for the missing feature, drain
replenishment, premature native certificate release and redundant publication
wakes. The native cancellation test's original assertions pass with exact
certificate retention. Tests also exercise actual private D-Bus following,
configured lock preference, machine suspend/resume fences, unavailable X11 helper
versus stale compositor hints, malformed/source errors, creator/observer panics,
pending stop, dropped/blocked creators, held cleanup and last-clone ownership.
Independent common tests require every native certificate and last clone before
admission reuse. Bounded interval/deadline/range properties reject invalid inputs
before factory exposure. No regression was removed, skipped or weakened.

Final locked/offline collection passes 750 host and 315 common library tests
(1065 total). Three pre-existing opt-in benchmark ignores remain unchanged.
All 123 scoped production functions pass individual 80% executable-line coverage;
none missing. Workspace formatting and warning-free offline compilation pass.
Whole-project cyclomatic checking passes 22,279 functions with none above 9.
The first diagnostic collection left connect at 79.4%; final evidence adds the
permanent unavailable-X11-helper case and is not mixed with diagnostic counters.
Production/build bytes stayed immutable from snapshot through native checking.

```sh
/tmp/blent-t1007-venv/bin/python scripts/coverage/rust_collect.py --output /tmp/blent-t1018-coverage-b --package blent --package blent-config --lib --feature accessory-transport
/tmp/blent-t1007-venv/bin/python scripts/coverage/report.py check --manifest /tmp/blent-t1018-coverage-b/manifest.json --lcov /tmp/blent-t1018-coverage-b/rust.lcov --native-sources /tmp/blent-t1018-coverage-b/native-sources.json --rust-platform linux --scope common/src/host_activity --scope host/src/activity --scope host/src/owned_task.rs --output /tmp/blent-t1018-coverage-b/per-function.json
cargo fmt --all -- --check
cargo check --offline --locked -p blent --features accessory-transport
/tmp/blent-t1007-venv/bin/python scripts/complexity/check.py
```

The controller/factory are library components; daemon capture/retained-monitor,
USB registration and auxiliary composition remain T1015/T1016/T1010, with Android
and joined/physical acceptance under T1011/T1009/T914. No forced desktop lock,
suspend/display-off, monitor-layout change, tablet interruption, production
service mutation, downloads, performance claim or distro-wide acceptance.
