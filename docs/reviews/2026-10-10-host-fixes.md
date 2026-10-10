# Host review fixes (2026-10-10)

## T831: apply coverage scope before parsing

Full inventory previously parsed the archived T790 C candidate and failed before
filtering out already-exempt documentation/benchmarks. Inventory now applies the
existing `coverage_source` policy first. No maintained-code exclusion or coverage
threshold changed. A permanent regression adds malformed exempt C/Rust fragments,
checks the maintained function remains inventoried, then proves malformed
maintained Rust still fails closed. All 69 coverage-tool tests pass; a full current
project snapshot succeeds. [Red/green logs](artifacts/2026-10-10-t831/).

## T821: independent selection values

Selection keys/results now depend only on portable negotiation values.
`EncoderSettings` owns key construction and exact matching, so selection data no
longer depends back on media implementation. Worker/cache/capture/input callers
use that single policy. Matching borrows decoder metadata, preserving the prior
allocation and comparison behavior; no performance gain is claimed.

The source-boundary regression failed before extraction and now passes. Permanent
property tests cover zero/odd/maximum unsigned geometry and scale, epoch wrapping,
metadata changes, explicit encoder choice, fallback, worker count and calibration.
Existing selection/cache/profile tests remain. Default host library/binary suite:
708 passing tests, seven existing ignored tests unchanged; all 89 production
functions in changed files meet individual 80% coverage, complexity <=9.
The six media tests also pass with all features in the already-installed local
Freedesktop SDK, using local Clang; network disabled. Host development headers
were unavailable, so this uses SDK libraries rather than claiming host-native
all-feature deployment acceptance. [Evidence](artifacts/2026-10-10-t821/).

## T811: responsive controller retirement

Camera/audio controller Drop now signals shutdown and transfers ownership to a
bounded reaper instead of joining on the caller. Explicit `shutdown` returns a
completion observation; completion means actual termination and join. The reaper
retains held workers and their admission slots, checks finished workers every
10 ms while retiring, and sleeps on its channel while idle. One stuck worker
cannot prevent another finished worker from being joined. Existing per-controller
command serialization still prevents overlapping session replacements.

`BLENT_CONTROL_WORKERS` configures the process-wide active-plus-retiring controller
budget (default 8, valid 1..64). This includes idle controllers, normally camera,
microphone and speakers. A larger budget tolerates more concurrent owners but
retains more native resources if cleanup stalls. Capacity exhaustion returns an
explicit unavailable error; it never authorizes premature release. Embedders may
inject one shared `WorkerPool` through `with_pool`. No forced cancellation of
native threads or recovery from process aborts is claimed.

Both permanent held-cleanup destructor tests first failed at the 200 ms caller
progress deadline. They now pass and retain capacity until cleanup and join;
replacement admission, sibling completion, panic reaping, repeated Stop, invalid
capacities and normal retirement pass. Existing cleanup assertions now await the
explicit completion contract. Full library suite: 303 passing tests, three
existing ignores unchanged. All 37 production functions in the changed modules
meet individual 80% coverage and complexity <=9.
[Red/green and source-attested coverage](artifacts/2026-10-10-t811/).

## T812: publish terminal controller outcomes

A shared outcome boundary catches session-factory and future-poll panics. Audio
and camera owners publish explicit failure and wait for another command. Audio
admission resets only after that terminal outcome. Stop cleanup errors remain
visible until a subsequent explicit action. An outer thread boundary also catches
runtime/supervisor/future destruction panics; a dead supervisor rejects Start
rather than leaving a false Starting state. Its owner must be retired before
constructing another. Native aborts remain outside unwind containment.

Permanent regressions first reproduced missing failure publication for both
controllers and hidden cleanup failure. They now cover setup/poll/cleanup/drop
panics, no automatic replay, explicit restart, successful completion and unknown
panic payloads. Full library plus integration suite passes; all 28 production
functions in affected modules meet individual 80% coverage and complexity <=9.
[Red/green and coverage evidence](artifacts/2026-10-10-t812/).

## T832: retain the complete distribution fixture

The existing T102 loader/distribution regression failed because its isolated C
fixture omitted `cursor_overlay.c`, which the current Makefile links. The fixture
now includes that unit; both permanent distribution tests pass offline. Production
code and packaging assertions are unchanged. Full-workspace RPM notice validation
was skipped by the maintainer for future RPM removal (T833); no dependencies
were downloaded. AppImage does not require `rpmbuild`.
[Red/green evidence](artifacts/2026-10-10-t832/).

## T824: independent persistence contracts

`persistence_contract` defines focused snapshot, transaction/cancellation and
settings merge/publication ports without paths, files or native service types.
FileConfig retains validation and merging; filesystem methods now belong to the
ConfigStore adapter. Host edit workers and GUI save/restart policy accept the
ports, permitting independent memory-store tests. Serialization, atomic replace,
merge-over-latest and publish-under-lock semantics remain intact. Restart occurs
after successful commit/publication and lock release. Cancellation before commit
rejects work; cancellation cannot roll back a commit already in progress.

Permanent structural checks first failed on model I/O and concrete consumer
dependencies. They now pass, alongside the same contract exercised against memory
and filesystem adapters, no-default-feature compilation, existing concurrent
merge/atomicity/cancellation regressions and GUI failed-save/restart-order tests.
A child-process test verifies live resolution preference reads from an isolated
store. The actual GUI starts and closes successfully under private Xvfb.
Linux functions in affected files meet individual 80% coverage and complexity <=9;
Windows-only functions remain visibly unmeasured under T737. Full workspace
library/binary tests pass; the broader integration run exposed the separately
fixed T832 fixture and missing RPM tool T833. No full-workspace pass is claimed.
[Source-attested coverage and red/green logs](artifacts/2026-10-10-t824/).

## T826: bounded asynchronous Wi-Fi persistence

Wi-Fi policy now awaits independent snapshot/transaction ports. A native storage
adapter uses a reusable process-wide pool: `BLENT_CONFIG_WORKERS` defaults to 2,
accepts 1..64, and is read once on first use. Each worker owns at most one active
call; the shared queue holds at most that many additional requests, and each
adapter admits at most two. Busy admission returns an explicit error. Larger
budgets tolerate more unrelated slow stores at the cost of retained threads and
native resources. Dedicated owners can inject a one-worker retirement budget.

Native calls run outside the queue lock and async event loop. Dropping a waiter
cancels queued work/cooperative lock waits, but retains admission and ownership
until native work really exits. Cancellation after commit starts cannot undo the
write. A request panic fails its reply without killing the reusable pool. Owner
retirement transfers held threads to the bounded reaper. The process-wide pool
keeps its workers idle until process exit; it does not repeatedly create threads.

Permanent red/green regressions cover a disposable held file lock while the
runtime heartbeat progresses, held reads, canceled queued edits, commit-in-progress,
sibling progress, repeated reuse, panic/error replies, capacity boundaries and
partial worker construction. Post-connect snapshot failures disconnect the attempt;
failed commits disconnect newly created routes while preserving an already-connected
route. Full host library and Wi-Fi CLI suite passes (315 + 3 tests, three existing
ignores unchanged). All 28 production functions in affected modules meet individual
80% coverage and complexity <=9. Caller cancellation after ADB connect still needs
attempt-level route ownership (T834); storage cancellation does not claim to solve
that separate lifecycle gap. [Evidence](artifacts/2026-10-10-t826/).
