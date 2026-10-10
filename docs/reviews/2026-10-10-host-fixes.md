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
