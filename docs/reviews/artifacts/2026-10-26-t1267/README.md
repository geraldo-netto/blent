# T1267: reserve complete native USB test fixtures

The fixture bound two independent ephemeral primary endpoints, then derived
extra video/input endpoints at +2 without checking or reserving those sockets.
A busy extra endpoint makes native startup fail and correctly retire peer 2's
unpublished slot. The controlled original bind-failure test reproduces that
missing-slot state and now records the complete error chain: input bind fails
with `Address already in use` (OS error 98). Admission exhaustion is independently
covered by the existing capacity-one child-process regression.

The new permanent allocation regressions first failed against the original
allocation behavior: an occupied derived endpoint was admitted, and extra slots
were not pinned. Allocation now validates and reserves every configured slot
before accepting a candidate. Primary guards are released immediately before
the actual fixed-port runtime bind; sibling guards survive primary startup and
are released before tests perform their intentional real binds. Candidate
selection has an explicit 64-attempt fixture-admission bound; capacities are
1 through 4. No production allocation, retry or retirement behavior changed.

The Reservation owns every socket until handoff/drop. Failed first/second extra
binds release all its partial sockets while leaving the independently held
conflicting listener untouched. Tests cover all four capacities, rejection on
either extra endpoint, partial failure and final guard release. Vec/RAII owns
cleanup on success, rejection or panic; no borrowed descriptor outlives its
owner. This fixture cannot promise exclusion of unrelated processes during a
close/rebind handoff. It removes admission of already-occupied derived ports;
it does not claim kernel reservation across that gap.

Verification, offline/locked using the installed SDK runner:

- Both permanent T1267 regressions failed before the allocation fix and pass
  afterwards. The original T908 startup assertions remain intact.
- Entire all-feature host main binary: 394 passed; 2 existing benchmarks ignored.
- Native USB cohort with error diagnostics: 20 passed, including stuck startup,
  bind failure, cancellation, panic recovery and controlled admission exhaustion.
- Final selected all-feature host run: 1465 top-level tests passed across library,
  binaries and mapping/audio/probe/Linux-lifecycle targets; 5 existing opt-in
  benchmarks ignored. Raw logs retain subprocess output separately.
- Every function in the changed test sources scores <=5 with the locked syn
  frontend. No production executable lines changed, so new production-function
  coverage is not applicable to this fixture-only correction.

The historical T1260 failure log contains no native startup error, so its exact
past OS error cannot be recovered. The reproduced fixture defect explains the
same observed missing-peer failure; this evidence does not establish a T1260
production retirement regression. Original historical failed evidence remains.
