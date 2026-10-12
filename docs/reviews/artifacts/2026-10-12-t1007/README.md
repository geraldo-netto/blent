# T1007: original broker exit recovers registration ownership

The Linux registration adapter captures a kernel process-lifetime handle from the
original authenticated IPC socket before registration exposure. A retained observer
survives worker panic, IPC failure and timeout. Remote release requires exact route
retirement, proven no exposure, or that original process's actual exit; capacity
additionally waits for actual local thread/runtime cleanup and endpoint release.
Process polling begins only after local native completion, not on active workers.

A portable BrokerLifetime contract owns proof semantics; the Linux lifetime adapter
uses SO_PEERPIDFD, with no numeric-PID lookup or reuse race. The OS socket and
registration adapters depend on that shared Linux module. If observation is
unavailable, a warning makes the limitation explicit and uncertain ownership stays
quarantined. Original death never restores authority or replays a registration.
Independent Android retirement and native USB helper retirement remain separate.

The permanent separate-process regression fails against pre-fix production sources
at 5bec93a0 after broker exit leaves capacity retained; it passes unchanged after
the fix. It checks listener ownership while the broker lives and listener/capacity
release after exit. Additional normal-suite tests cover a held native local thread,
a replacement broker at the same path using identical request/route numbers,
unrelated process exit, repeated stale close, missing observers, invalid descriptors,
live-process socket closure, close-on-exec and portable failure-state preservation.

Final fresh locked/offline host/common library collection passes 712 host tests
and 304 common tests. Three pre-existing opt-in host benchmark ignores remain;
none were introduced. All 54 scoped production functions pass individual native
80% executable-line coverage, none missing. Every reviewed production/test function
(117) has complexity at most 9; the whole-project complexity gate passes 22,061
functions with none above 9. Sources and the collector were frozen from snapshot
through the native check; source/build hashes, executable fingerprints, exact
Cargo command, LCOV counters and report hashes are retained.

The initial diagnostic collection found an untested unavailable-observer branch;
a substantive permanent rejection/quarantine regression closes that gap. Its
counters are not mixed with final evidence. The workspace formatting check exposes
pre-existing drift in eight files, retained in workspace-format.log.gz and tracked
as open T1012. It is not a passing format result.
No downloads, production service changes, USB/ADB, physical host suspend/lock or
tablet interruption were performed. Physical activity/media acceptance remains
postponed; T1008–T1011 own the remaining runtime integration and T914 acceptance.
