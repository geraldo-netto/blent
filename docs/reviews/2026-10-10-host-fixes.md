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
