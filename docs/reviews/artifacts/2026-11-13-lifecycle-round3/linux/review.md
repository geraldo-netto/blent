# Linux startup and activity recovery review

Reviewed HEAD `31912bc7`. Source and isolated probes only; no production/test
changes during review. Three confirmed findings were sent to the parent ledger. The third emerged
while checking the first finding’s replacement-initialization dependency.

## T1427 — transient invalidation refresh failure never retries

`host/src/activity/native.rs:302–308` publishes unknown after invalidation, then
accepts `refresh()` returning None. Lines327–340 collapse timeout, D-Bus and
conversion errors into None. The live actor then waits only for new signals or
owner loss. No replacement request or refresh retry remains scheduled.

`refresh_failure.rs` uses public Factory/Following against a fresh private D-Bus.
The compositor's initial Get succeeds. One invalidation is followed by a one-shot
Get error. Five configured recovery intervals later, publication still denies
display demand with DisplayUnknown, epoch1/revision2; only the initial and failed
Get occurred. A direct query to the same original owner succeeds with PowerSaveMode0
while publication remains unchanged. A valid signal control restores demand.
`refresh-failure.log` retains exact output. Source establishes the missing retry;
the finite wait alone is not claimed as proof of permanent loss.

Severity medium, effort small. Recovery must retry observation or use the existing
replacement workflow, retain unknown demand until fresh authority returns, and
settle the original native attempt before reuse. Permanent coverage must include
invalidation followed by transient query failure and recovery, continued failure,
and explicit stop. Both lock and display share the affected refresh path.

## T1428 — failed startup leaves its PID publication

`host/src/linux_main.rs:1690` writes the PID, while removal at1909 is reached only
by normal shutdown. Later `?` paths skip cleanup. In an isolated user/mount/PID
namespace, the actual daemon with BLENT_ACTIVITY_LIMIT0 exits1 after writing PID3;
the PID file still exists. `startup_pid.py` reuses the existing fake-only startup
fixture, and `startup-pid.log` retains output. It mounts private /dev and sets
private HOME/runtime/configuration, fake-only ADB PATH and nonexistent D-Bus
addresses. No physical device or running desktop service is accessed.

Severity low, effort small. Retain identity-bound PID cleanup through successful
shutdown, error, cancellation and unwinding, without removing a successor's PID.
Permanent regressions must cover failed startup and replaced-PID preservation.
Status/stop can currently clean the stale file later; this is stale publication,
not proof that an unrelated process can be killed or resources are leaked.

## T1429 — failed initial property query leaves a healthy-looking unknown source

`native.rs::boolean` converts property errors into `Flag::default`; logind
initialization still declares power/lock capabilities supported. The resulting
actor waits only for future signals and never retries the failed initial query.
`initial_query_failure.rs` reproduces this independently with public PowerFactory
and Following. A single initial PreparingForSleep query fails; the same owner
then answers false successfully. After five configured recovery intervals,
publication remains PowerUnknown, both demands false, with exactly one property
query. A direct successful query leaves publication unchanged. The exact trace is
`initial-query-failure.log`; the source establishes the absent retry scheduling.

Severity medium, effort small. Retry transient failed initial snapshots without
claiming authoritative running state or discarding already admitted capabilities.
Preserve explicit unsupported/no-owner behavior. This also governs replacement
initialization after T1427. Permanent coverage must distinguish initial transient
query failure, prolonged failure/recovery, and explicit unsupported properties or
absent owners, with original native settlement and stop control.

## Other reviewed invariants and limits

Examined signal registration/latching, repeated-signal behavior, startup error
classification, process admission/cutoff serialization, runtime drain after
return/unwind, session/task/native proof retention, source creation/loss/retry,
capability continuity and downstream activity publication. The final focused pass
found no additional evidence-backed issue in those paths. Stop suppresses later
publication; retries retain original attempt completion and demand fresh epochs;
source capability disappearance after admission remains conservative.

Executed three review probe programs: two private-D-Bus public API probes and one
actual-daemon namespace probe. No new coverage campaign or physical suspend,
EVDI, USB, input injection, live configuration or service change was performed.
All probes exited and reaped their owned children; Following retired before its
private server closed. Their temporary namespace roots were removed. Executables
and LLVM profiles are external under `/tmp/blent-lifecycle-round3-linux`.

All 19 authored probe functions measured: Rust17 maximum complexity3, Python2
maximum2 (`probe-complexity.json`, project syn2.0.119/Python AST tools). Artifact
and relevant source hashes are in `review-hashes.json`. These are finite fixture
results, not claims about complete hardware or platform support.

## Reproduction

Compile `refresh_failure.rs` externally with rustc edition2021, instrument-coverage,
and --extern matching existing Blent c16265474e6be6f1, blent_config e8fb2a5f0cf99980,
zbus240d4e1d9a0efdaf and tokio9a9d4a755c6dd115 rlibs under target/debug/build.
Pass dependency search paths for target/debug/deps and each target/debug/build
package out directory; do not mix dependency fingerprints. The previous review's
`lifecycle-recheck/linux/review.md` records the full equivalent compilation command.
Run:

```
dbus-run-session -- env LLVM_PROFILE_FILE=/tmp/blent-lifecycle-round3-linux/refresh-%p-%m.profraw /tmp/blent-lifecycle-round3-linux/refresh-failure
/usr/bin/unshare --user --map-root-user --mount --pid --fork --mount-proc /usr/bin/python3 docs/reviews/artifacts/2026-11-13-lifecycle-round3/linux/startup_pid.py target/debug/blent
```
