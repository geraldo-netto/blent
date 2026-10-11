# T995: admission follows actual original native retirement

The UI-facing AudioController contract remains independent of native adapters.
Report observes portable Completion proofs through a fixed pair of slots per
direction: original USB owner plus isolated helper. Linux records proof directly
after creation. Worker supervision waits for actual completion before releasing
admission on errors, success or panics. Stop/Drop remain bounded at the caller;
that worker's completion and Start reservation remain pending until cleanup.
Other directions remain responsive. Completed slots may be reused; pending ones
cannot be replaced or grow history. CLI drops the connection owner before waiting
for its proof, avoiding a self-held retirement barrier.

Permanent red/green regression reproduced premature Start admission after a
backend error with held proof. The same regression passes; success/panic variants
retain admission and sibling control progress. Capacity/replacement cases prove
that timeout/requests never complete observation. An owned panicking future test
proves its native guard actually drops before waiting. Existing real local helper
held-lease tests now verify tracked completion remains pending at pipeline error;
private real TLS speaker routing records both original owner and helper proof.
All existing controller lifecycle/panic/drop tests remain unchanged.

Final fresh offline accessory suite: 41 host + 49 common audio tests pass.
Default: 38 host + 49 common pass. All 46 affected production functions meet
individual 80% executable-line coverage; no missing counters or exclusions.
Changed Rust complexity: 113 functions, maximum nine. GUI audio controls also
compile and their existing tests pass. Final collections use identical immutable
inputs and exclusively owned fresh targets. Compressed manifests, attestations,
LCOV, source inventories, counters, red/green and owned-panic/GUI logs retain the
evidence. Earlier counter collections were discarded after the final owned-panic
test changed the build inventory; no stale counters were reused.

No downloads, installs or live host/tablet tests. Physical audio acceptance remains
T860/T859; SDK-wide native-audio qualification T835 and Windows validation T902.
Attestation includes the pre-existing uncommitted TLS fixture; unrelated edits
remain unstaged and are not part of this commit.
