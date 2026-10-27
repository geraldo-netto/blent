# T1080: independent auxiliary activity workflow

Auxiliary media waits for its original native work to settle before resuming after machine-power changes. Bounded models and actual worker-cancellation tests passed; individual backend integration remained separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The new portable workflow uses the independent WorkObservation/Auxiliary policy
and read-only publications. Each invocation retains its own manual intent and
original run. Lock, display-off and explicit unsupported/absent power observation
preserve legacy work. Suspend, unavailable power, known-source loss and coalesced
stop history withdraw the original attempt. Source resumption alone cannot create
a replacement while original native settlement is withheld. Ordinary completion,
errors and factory/polling panics settle once and return without automatic replay.
Late creation receives the original stop; closing manual ownership denies resume.

The caller owns the workflow and native admission budget. A start callback owns
one attempt; its stop receiver is borrowed for that attempt. Source/manual cutoff
sets the original stop channel; native quiescence comes from the separate settle
callback after the attempt returns. That callback's independent contract requires
actual original obligations to settle, including partial failures. New admission
rechecks current source/manual state only after that barrier. Desired observations
grant neither native acquisition nor device consent. Linux composition and actual
camera/audio authority remain T1081–T1083; joined acceptance remains T1010.

Dropping the workflow closes its original stop sender. Native adapters must retain
uncertain work independently if their future is dropped. The permanent cancellation
case uses an actual blocking worker, OwnedTask and a separate admission retained as
a dependent lease: outer cancellation does not release capacity, a closed original
stop is observed, and only actual thread join/dependent cleanup releases the slot.
The external memory-only gate also settles on sender loss. Another boundary case
requires the completed attempt future to be destroyed before native settlement.
This does not prove arbitrary native adapters safe; each integration retains its
own completion obligations and validation.

There is at most one attempt/stop channel and one armed source notice per workflow;
loop-local borrowed notices/futures are dropped on wake or select cancellation.
The tests bound retained originals to 16 (six-transition histories use at most 4),
and require all original obligations to settle before replacement and at cleanup.
Conditional eventual cleanup assumes continued polling, delivered original stop
and eventual native evidence. A permanently missing native fence remains owned
and funded, blocking reuse. The 64 serial six-transition histories and selected
coalescing/fault/native-cancellation cases do not prove all interleavings.

Both deliberate faulty controls fail: skipping settlement admits a replacement
before its predecessor is settled, and ignoring original stop history misses a
coalesced suspend/resume. These are controls, not preexisting production defects.
An early invalid-fixture draft also incorrectly rejected a valid revision-zero
snapshot. The final fixture mutates an actual future stop fence and retains a
separate valid-zero-boundary test; production validation was not weakened.

Fresh exclusively owned offline LLVM collection passes all 15 permanent tests,
including eight malformed/lost snapshot variants, pending/failed/stopped sources,
late creation, original failures, held cleanup and actual worker cancellation.
Every Rust source hash matches the collection manifest. All five new production
functions pass individually: four 100%, stopped 87.5% (its channel-closed return
line is unmeasured in this collection). Raw LCOV, native object attestations and
source/tool evidence are retained. Kotlin/Rust aggregate coverage is not substituted
for this gate. The source audit covers 40 production/test functions, maximum source
estimate 9, with cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2. Native SonarQube
remains T1019. All 12 architecture checks pass. No physical operation or transfer.

</details>
