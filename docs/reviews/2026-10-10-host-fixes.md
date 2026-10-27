# Host review fixes (2026-10-10)

Host fixes separate public contracts from adapters, supervise worker failures, and bound command output while retaining ownership until native cleanup finishes. The report records individual regressions and coverage scopes, including later corrections found during verification. These results do not certify every platform or a general performance gain.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

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
code and packaging assertions are unchanged. Full-workspace Packaging notice validation
was skipped by the maintainer for future package cleanup (T833); no dependencies
were downloaded. AppImage does not require the external package builder.
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
fixed T832 fixture and missing package tool T833. No full-workspace pass is claimed.
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

## T820: independent capture contracts and owned outcomes

`capture_contract` defines immutable packet publication, codec-header publication,
keyframe requests, viewer observations, generation-bound profile leases and owned
worker futures. It contains no queue, attachment, codec-cache, latency-tracker,
native capture or task-handle implementations. Session adapters implement those
ports. Concrete stream/input bindings remain in the composition-owned
SessionResources bundle, outside the backend contract. Linux construction connects
its manager resources there; an independent fake backend can initialize headers
and stream entirely through its supplied ports.

The session binds both listeners before invoking capture and owns spawning all
returned work. Success/error outcomes survive the task boundary; logging no longer
erases errors. T813 separately adds active failure supervision. Selection uses a
profile lease interface; shared frame bytes and generation ownership are preserved.
There is one boxed profile lease per cache attempt, not per frame. No frame data
copy or hot-path allocation was added by packet publication.

The structural check failed before extraction. Independent fake and real-adapter
regressions preserve byte addresses, rejected ownership, keyframe demand, header
clearing, viewer epochs and attachment retirement. Existing listener-before-start,
authentication, generation, worker cancellation and packet limits remain tested.
Default host library/binary collection passes 725 tests, with seven existing
ignores unchanged; final focused session/contract checks also pass. Installed SDK
capture/encoder/session suites and portable contracts pass with all features;
broad SDK native fixture limitations remain T835. A permanent stock-libavcodec
submission test compares uploaded CPU test surfaces with direct frames, including
keyframe packet equality; this tests Blent's dispatch without claiming GPU-driver
acceptance. All 184 production functions in scoped changed modules meet individual
80% coverage and complexity <=9. No coverage exclusion was added.

The retained release microbenchmark runs twelve alternating 200,000-iteration
samples with immutable 4 KiB packets: median publish/read is 50.67 ns directly and
52.30 ns through the port (1.64 ns added). This is interface overhead, not an
end-to-end improvement or display latency measurement.
[Source-attested evidence](artifacts/2026-10-10-t820/).

## T813: supervise session failures and revoke failed attachments

One session supervisor owns capture, stream, input, display-gate, shutdown and
capture-auxiliary task handles. Error, panic or unexpected successful completion
retires that session's attachment and credentials, requests sibling shutdown and
publishes a named failure through an independent lifecycle value. Native input
remains a separately scheduled, owned task; its terminal result now propagates to
the input server. Other sessions retain their own workers and generations.
Transport polling cannot reactivate a retired attachment. Retry requires a new
session owner; no automatic retry was added.

Both listeners are bound before capture construction; no task is spawned until
the backend returns its owned futures. A constructor panic returns an error and
drops the unstarted listeners. Explicit shutdown reports Stopped after task
cleanup. Owner drop reports Cancelled, meaning cancellation was requested without
confirming native retirement. Failed state survives subsequent shutdown. If a
worker panics while holding its generation lock, retirement recovers the poisoned
lock only to revoke authority and clear metadata, never to resume partial state.

Permanent T813 regressions were observed failing for capture error/panic/early
success, constructor panic, stale Running after owner drop, erased native-input
completion, late credentials and poisoned cleanup. Additional tests cover every
service outcome, independent sessions, expected shutdown, bounded malformed/late
transport values, repeated retirement and the existing async capture grace.
Authentication tests retain their assertions; their fixture now supplies an idle
input backend instead of dropping an unstarted native capture owner whose channel
closure terminates input. Existing loopback authentication and lifetime tests stay
in the normal suite.

Control-side bounds during native shutdown, cleanup-phase error reporting and
retention of unconfirmed resources remain T814/T819. Task abort is not evidence
that a started native call stopped. No production services were reloaded, no
packages downloaded and no external dependency code changed.

Final default host collection passes 744 tests with seven existing ignores. The
installed offline SDK passes all 33 scoped all-feature session/contract tests;
broad SDK prerequisites remain T835. All 59 maintained production functions in
the changed session, supervision, attachment and input modules exceed or meet
80% individual executable-line coverage and have complexity <=9. Source hashes
match the collection manifest. Four existing architecture guards also pass.
[Retained red/green and coverage evidence](artifacts/2026-10-10-t813/).

## T814: bound control waits and retain unfinished ownership

Owned commands/media children, session supervisors, capture tasks and native
control/config workers reserve capacity before spawn. Every control retirement
phase now has a deadline. Unknown completion transfers the owner to bounded
retirement observation instead of detaching it or freeing its admission. Capacity
is released only after actual completion and dependent cleanup. Capture FIFO
leases and camera device locks remain owned through incomplete native retirement;
replacement capture checks its pending completion tickets first. In-process
encoder tasks retain their input lease and buffers even after task abort.

The shared observer does not join native threads or destroy potentially blocking
dependent guards itself. Each retiring owner may use one isolated cleanup worker,
retaining the same admission. A permanent regression demonstrates why this is
needed: `JoinHandle::is_finished` can precede thread-local destruction. Held TLS
cleanup and held lease destruction no longer block sibling retirements or control
return. Cleanup panic quarantines the slot. Session/encoder cleanup errors and
panics propagate instead of being reported as successful stop; late session
cleanup publishes failure while preserving an earlier failure.

Persistence cancellation uses nonblocking stop admission. A transaction already
committing keeps its atomic operation/result while control shutdown returns;
queued transactions are cancelled. Camera producer collection has its own bound.
Settings and tradeoffs are documented in [host retirement bounds](../architecture.md#host-retirement-bounds).

Seven permanent regressions were observed red before their fixes; additional
capacity, replacement, cancellation, lease, sibling-progress and malformed-limit
properties remain in normal suites. Default collection passes 954 tests with
seven existing ignores; the added stop test, 38 optional capture tests, ten audio
unit tests, two private native-audio acceptance tests, 128 no-default common
tests and four architecture guards also pass. All 213 scoped production functions
meet individual 80% executable-line coverage and complexity <=9. The installed
SDK lacks the PipeWire executable, so native-audio-only binaries built there ran
the existing private acceptance suite with installed host tools. No downloads,
external-code changes or production reloads were needed.

These are control-side bounds. Started blocking work still cannot be aborted;
optional codec process isolation and final Tokio runtime destruction remain T819.
Command output limits remain T815; Windows-native job/retirement evidence remains
T737 and broad SDK native-fixture prerequisites remain T835.
[Red/green logs, source hashes and per-function evidence](artifacts/2026-10-10-t814/).

## T815: bound command output while collecting

Commands collect through bounded kernel pipes and one admitted native worker,
alternating 16 KiB reads between stdout/stderr. Independent stream and combined
byte budgets are checked before buffer growth; overflow is an explicit error.
There are no unlimited temporary output files and async callers do not perform
synchronous final reads or allocation. The shared native-thread retirement
adapter is now reused by both control workers and collectors; held collection
retains its slot through actual cleanup without holding its caller.

Collector errors race stdin delivery as well as child completion, so a child
that ignores stdin cannot hide output overflow behind a blocked input write.
After direct-child exit, a final available-byte drain avoids waiting for inherited
writer handles. Reaped children are never signalled by a late collection error.
The read pass observes child completion before probing pipes, preventing a final
write/exit between the idle probe and completion observation from losing bytes.
Interrupted reads retry; failures/panics preserve explicit errors. Existing process
tree cancellation and successful-descendant behavior remain tested.

Defaults are 4 MiB stdout, 1 MiB stderr, 5 MiB combined and 128 active/retiring
collectors, all configurable with validated bounds. Buffers grow on demand and
outputs are moved to callers. Allocation-capacity and inherited-output tradeoffs
are documented in [command output limits](../architecture.md#command-output-limits).
Unrepresentable command deadlines and malformed byte/capacity settings fail before
child creation. The separate public retirement-grace overflow discovered during
boundary checking is T836, with its own fix/commit to follow.

Five permanent regressions were observed failing before their corresponding fixes;
held collection restores old caller-thread behavior through the new reader port
for its red check. The final default collection passes 977 tests with seven
existing ignores. All 47 scoped production functions meet individual 80% coverage
and complexity <=9. The 128 no-default common tests, four architecture guards and
Windows GNU cross-check also pass offline; native Windows acceptance/counters remain
T737. No production reload or downloaded resource was needed. No measured throughput
or end-to-end latency improvement is claimed.
[Red/green and source-attested coverage](artifacts/2026-10-10-t815/).

## T836: reject unrepresentable retirement grace

`OwnedChild::retire` uses checked monotonic-clock addition for grace and reap.
An unrepresentable request returns `RetirementError::InvalidGrace` with incomplete
phase; owned Drop still signals and retains the child/leases until real cleanup.
Valid requests retain their existing grace/reap behavior. Signal failures now use
the same independent typed error contract instead of an untyped signal-only field.

The permanent extreme-grace regression was observed panicking before this change.
It now passes together with valid boundary durations, cancellation before polling,
final lease release and accepted/denied-signal capacity tests. All 228 common tests
and 13 focused host session tests pass; Windows GNU cross-compilation passes.
All 12 changed production methods have >=80% individual coverage and complexity
<=9. Strict merging exposed truncated profiles from intentionally killed T815
producer fixtures (T837); the scoped report uses complete profiles through LLVM's
supported merge mode, retaining the invalid records and every production method.
T837 separately restores clean strict collection; no coverage exclusion is added.
[Evidence and source hashes](artifacts/2026-10-10-t836/).

</details>
