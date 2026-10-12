# T1013: independently owned X11 DPMS observations

The common DisplayObservation wire schema is independent of native X11 types.
The Linux source keeps D-Bus power/lock and Wayland compositor actors; X11 uses a
read-only isolated helper dispatched by the existing executable. Runtime-loaded
Xlib/Xext functions retain their libraries through display close. Native booleans,
status and power levels are validated; disabled DPMS ignores its undefined level.
Missing libraries/display/extension/capability remain explicitly unsupported.

Typed snapshots use the original stdout pipe, bounded to 256 bytes before decoding.
The heartbeat interval follows the configured deadline, capped at 250 ms. Source
loss clears only its own display state. Logical stop is separate from actual
creator/child retirement; startup timeout cannot lose a late native child.

The feature test initially fails because the independent schema is absent. The
permanent heartbeat regression then fails against the fixed 250 ms heartbeat and
passes with deadline-derived pacing. Tests are retained in the normal suites.
Bounded native properties include every u16 power level and invalid boolean/status
boundaries. Native ABI fixtures cover all levels, disabled/unsupported capability,
failed queries and exactly-once close. Private Xvfb tests execute installed Xlib
and the real helper dispatcher. A private TCP server holds a real Xlib connection;
the test proves its original process remains owned until actual retirement.
Pipe fixtures cover transitions, framing/schema/timeout/loss and independent
power/lock fields. A held creator returns a late child after startup timeout and
stop; both proofs must complete before source retirement.

Final locked/offline native collection passes 739 host library, 441 host binary,
309 common library, two DPMS integration and two retained Linux lifecycle tests
(1493 total). Seven pre-existing opt-in benchmark ignores remain unchanged.
All 150 scoped production functions pass individual 80% executable-line coverage;
none missing. The whole-project cyclomatic gate passes 22,208 functions, maximum 9.
The first diagnostic collection omitted Linux daemon integration tests and is not
mixed into final evidence. The final immutable collection includes those existing
tests and fingerprints all production/build sources and native ELF objects.

Commands:

```sh
/tmp/blent-t1007-venv/bin/python scripts/coverage/rust_collect.py --output /tmp/blent-t1013-coverage-b --package blent --package blent-config --lib --bins --test activity_dpms --test linux_lifecycle --feature accessory-transport
/tmp/blent-t1007-venv/bin/python scripts/coverage/report.py check --manifest /tmp/blent-t1013-coverage-b/manifest.json --lcov /tmp/blent-t1013-coverage-b/rust.lcov --native-sources /tmp/blent-t1013-coverage-b/native-sources.json --rust-platform linux --scope common/src/host_activity --scope host/src/activity --scope host/src/dpms_probe --scope host/src/linux_main.rs --output /tmp/blent-t1013-coverage-b/per-function.json
/tmp/blent-t1007-venv/bin/python scripts/complexity/check.py
```

No downloads, installation, production service changes, USB/ADB, forced desktop
display-off/lock/suspend or tablet interruption. These tests establish adapter
and ownership boundaries, not physical monitor transitions, distro-wide support,
runtime power savings or joined daemon/media integration. T1009–T1011/T914 retain
that integration and physical acceptance. D-Bus remains the chosen Linux
power/session interface regardless of distribution.
