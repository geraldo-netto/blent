# Selected follow-up implementation

Owner scope: T1740, T1710/T1711/T1625, T1529/T1777/T1772, T1495 and T1795.
One verified local commit per item. No push, installation or reload is authorized
by this batch; installed sessions remain untouched. T1740 is decomposed into
T1808 toolchain and T1809 GUI migration with parent integration retained.

## T1710 — canonical service template

`scripts/blent.service` is the sole authoritative unit template. Arch packaging
uses the same literal-path renderer as source and AppImage installation; portable
staging no longer ships a duplicate. A changed-template fixture proves the package
consumes the canonical policy and checks rendered bytes and file mode. Both new
regressions failed before removal/consolidation and pass afterward. Existing
cleanup/ADB policies and path assertions remain intact. The affected six
source-build/user-path tests pass after the separate T1810 fixture correction.

The maintained package function executes all 23/23 measured shell lines. No native
session or new retained process owner is created: package() writes only within
its caller-owned staging directory. Native retirement fixtures use private units;
installed Blent services are untouched. Broader installation integration remains
T1625. [Evidence](artifacts/2026-11-22-followup-batch/t1710/).

## T1711 — shared privileged setup

Source/tarball, Make and Arch hooks delegate to `scripts/setup-system.sh`. The
canonical resource templates are staged with it. Existing EVDI configuration and
live devices are preserved; owned uinput rules refresh consistently. Configuration
write failures stop activation; GPU, udev and safe EVDI activation failures warn
without discarding installed configuration. AppImage user registration stays
unprivileged. There is no retained Debian package hook in the current tree.

New route/behavior regressions failed before consolidation. All 58 focused
installation tests pass afterward, including existing capacity, GPU ordering and
no-unload tests. All 11 maintained shell functions have 100% executable-line
coverage, using exact source hashes or body-hash-attested fixture extraction.
Complexity is at most 9 across the changed source/test files; measurement uses
the repository tree-sitter checker and Bash DEBUG counters. No native root setup
was performed. Temporary config publication owns one same-directory file until
synchronous install/link settlement; EXIT removes that exact temporary, and
no-clobber linking preserves concurrent external ownership. No background owner,
retained callback or unbounded buffer is introduced. Fault tests cover copy
failure, existing symlinks and concurrent publication.
[Evidence](artifacts/2026-11-22-followup-batch/t1711/).

## T1808 — packaging compiler prerequisite

Pin official `rust:1.100-bookworm` at manifest-list digest
`ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea`,
verified through Docker Registry's image manifest. Official amd64 source revision:
`rust-lang/docker-rust` `3f4c9cd699d3671ee657c75b5e592f6eb0b93824`. The complete
packaging image builds; its native compiler/linker probe runs with Rust 1.100.0,
glibc 2.36, installed EVDI 1.15.0, libusb 1.0.26 and PipeWire 0.3.65. This preserves
the Debian 12 ABI baseline while meeting eframe 0.37's Rust 1.95 minimum. The
permanent pin/minimum/documentation fixture failed at 1.90 and passes afterward.
This is toolchain qualification, not a full application or distribution build.
No production function or native session ownership changes.
[Evidence](artifacts/2026-11-22-followup-batch/t1808/).

## T1625 — installation integration

All current routes consume the canonical service template and shared privileged
setup where applicable. Added permanent Arch resource-byte checks and portable
bundle checks for the shared setup and absence of a duplicate service template.
The combined run passed 109 executed tests across source, Make, package staging,
AppImage and setup fixtures; one mistyped module name failed discovery, then the
actual identity module passed separately. All four artifact/parity tests also
pass with the new assertions. No assertion was removed. Changed test functions
pass complexity <=9. These isolated fixtures cover files, outcome policy and
private native-loader behavior; no pacman transaction, privileged setup or live
service reload was performed. Existing native module acceptance remains separate.
[Evidence](artifacts/2026-11-22-followup-batch/t1625/).

## T1740 / T1809 — evaluated candidate, not landed

The eframe 0.37 adapter candidate passed all 304 GUI unit tests, both process
lifecycle integration tests and the startup/accessibility integration test. A
further 16 native tests passed after explicitly retaining Wayland client
decorations. The release build and GUI Clippy pass; all 16 maintained production
functions have >=80% measured line coverage, and 618 measured source/test
functions have complexity <=9. Exact LLVM evidence combines the two executed
GUI objects with identical maintained source and differing feature selection.

The candidate handles the new separate logic/UI callbacks, including hidden
activation and terminal-preview cleanup (both new regressions failed before that
adaptation). Headless tests explicitly consume texture deltas. Fixtures retain
their original safety assertions: framework theme initialization is settled,
slider numeric text is located separately from its suffix, animation preferences
are set after selecting the test theme, and the notice-only viewport fixture
starts with no unrelated draft. An intermediate run rebuilt its executable during
self-spawning tests; those invocation failures were eliminated using an isolated
coverage target. Final tests do not exhibit those failures.

**The candidate does not satisfy the dependency cleanup objective.** In the
Linux GUI normal/build closure, distinct package/version pairs rise from 309 to
340; package names with multiple versions rise from 12 to 14. The old AccessKit
hashbrown 0.15 edge disappears, but the newly enabled Wayland clipboard data-control
path introduces `arboard → wl-clipboard-rs 0.9.5 → tree_magic_mini 3.2.2 → petgraph
0.8.3 → hashbrown 0.15.5`. Registry checks confirmed current petgraph/tree_magic/
wl-clipboard releases retain that requirement; arboard 3.7 retains it as well.
The selected API migration was therefore not landed. Current GUI dependencies
and code remain in place; T1740/T1809 retain the precise upstream prerequisite.
No staged bundle notices were relabelled for an unshipped candidate.

[Graph and recoverable patch identity](artifacts/2026-11-22-followup-batch/t1740/)
and [candidate validation](artifacts/2026-11-22-followup-batch/t1809/). Installed
processes and artifacts remain untouched. The packaging compiler improvement
T1808 remains independently verified and committed.

## T1811 — remaining Rust installation fixtures

The common test build exposed two compile-time references to the deleted unit
copy; the real-Cargo Arch fixture also failed because it omitted shared setup.
Both policy fixtures now read the canonical unit, retaining all deadline, kill
escalation and scheduling assertions. Arch stages the actual canonical renderer,
unit and setup script; its original fresh-output assertions pass for environment
and Cargo-config output overrides. Two policy tests and both Arch tests pass.
Changed test functions pass complexity <=9; no production function changed.
[Evidence](artifacts/2026-11-22-followup-batch/t1811/).

## T1529 — bounded idle observation backoff

Retirement waits grow from 5 to 100 ms and reset on admission/completion. Empty
and entirely quarantined pools block for new work. Cleanup exposes a terminal
quarantine only after its original worker has joined; it never releases the
unknown obligations. Command/output loops preserve the first eight 1 ms idle
polls, then grow to a 10 ms cap; output progress resets the cadence and caller
deadlines clip sleeps. No native readiness or completion proof is inferred from
the delay. Existing admissions, pipe capacities and original stop/join ownership
remain authoritative; backoff stores only bounded scalar state per loop.

Two permanent observation-count regressions failed before the change (198 reaper
observations in one second; 477 idle reads in 500 ms). All 521 common library tests
pass, including panic, timeout, cancellation, native TLS teardown and capacity
regressions. All 13 maintained functions have >=80% line coverage; 100 measured
source/test/benchmark functions have complexity <=9. Common production Clippy passes.

Finite native benchmark (`common/examples/control_polling.rs`, strace 6.8):
- Two-second retained child: wait4 calls 394 → 25; futex calls 396 → 27.
- Two-second captured command: wait4 1822 → 210; reads 3565 → 435.
- Quarantine-only workload: one blocking futex receive, no periodic observer.
- Three runs of 50 short commands: baseline p50 2293–2428 µs, p95 2390–2572 µs;
  final p50 2246–2281 µs, p95 2299–2377 µs. An initial candidate raised p95 to
  about 4.6 ms; the initial fast observation window avoids that measured regression.

These local syscall/latency measurements are not tablet power measurements or
real-time guarantees. Worst-case observation adds up to 100 ms for retirement
and 10 ms for command/output control. A separate reporting-panic gap found while
reviewing this owner is retained as T1812.
[Evidence](artifacts/2026-11-22-followup-batch/t1529/).

## T1812 — contain retirement reporting failures

A permanent subscriber-fault regression failed before the fix: the reporting
panic escaped `Record::poll`. The record now contains reporting failures and
retains both original panic payloads under its existing admission. Neither
arbitrary payload destructors nor the failed observer run again. Capacity stays
quarantined; an independent sibling still retires. No new native owner or queue
is introduced: at most one adapter/report failure pair belongs to each admitted
record, retained until process exit because its obligations remain unknown.

All 522 common library tests pass. Both maintained production functions exceed
80% line coverage; all 30 measured functions have complexity <=7. Common Clippy
passes. [Evidence](artifacts/2026-11-22-followup-batch/t1812/).

## T1813 — wait for failed bootstrap ownership in the restart fixture

The existing repair/restart test reproduced a premature retry of the one-slot
shared authority domain. It now waits for that guarded domain's original
admission to settle before retrying. All existing identity, confirmation, trust
and restart assertions remain; the unchanged production admission still rejects
reuse while native retirement is pending. The test passes after failing with
`Resources still active or retiring`. Changed code is test-only; measured
functions remain within complexity 9. [Evidence](artifacts/2026-11-22-followup-batch/t1813/).

## T1777 — schedule broker maintenance by actual deadlines

The dispatcher now chooses the earliest accept retry, controller expiry,
eligible automatic retry or inventory deadline. Live sockets/channels supply
readiness wakes. Finished inventory and closing native owners retain a bounded
100 ms observation cadence, including controller tombstones after their adapter
task leaves. Timer recalculation cannot postpone that retained absolute deadline.
No new native owner, queue or allocation is introduced; exact original completion
still gates collection. Suppressed retries and unavailable authority/inventory
produce no retry timer.

The live-idle-client regression failed with 31 wakes in 600 ms and passes with
zero. All 307 selected broker tests pass; the separately corrected T1813 repair
fixture also passes. Existing accept retry, native collection, automatic retry,
expiry, channel and TLS regressions remain. All 17 maintained functions exceed
80% line coverage; all 69 measured functions have complexity <=7. Host Clippy
still fails only on the previously recorded T1801 inventory return type; this
batch does not suppress it. Native USB tablet/battery behavior was not measured.
[Evidence](artifacts/2026-11-22-followup-batch/t1777/).

## T1772 — integrate broker dispatcher reductions

All 310 broker tests pass together, including native accept rejection/retry,
approval/setup expiry, bounded automatic retry, inventory freshness, TLS routes
and exact original retirement. New permanent integration checks measure zero
steady pending-turn allocations/reallocations across 256 turns with zero links
and with one authenticated link. Both empty and authenticated idle dispatchers
record zero wakes over isolated 600 ms windows. The authenticated case retains
its native admission throughout observation and verifies its release after
shutdown. Its link uses the production TLS/control path over a hosted duplex
stream and an explicitly owned fixture admission, not USB hardware. These are
outer-dispatcher results; inner link polling T1773 and tablet power remain
unmeasured. No production function changed; measured test complexity <=9.
[Evidence](artifacts/2026-11-22-followup-batch/t1772/).
