# T644: current diagnostics pass; historical cause evidence is missing

The original ledger at61c7d497 and diagnostic README explicitly require the
original runner's live-child evidence or an equivalent causal reproduction.
The later c615d103 reopening identified installed local tools, which permits
current research but does not supply the missing historical observation.
Original run36276441908 attempt1 retained empty stderr and no child status,
wait-channel, syscall or completed display-publication trace. Those missing
observations cannot be recovered from a current successful run.

Current offline command python3 -m unittest
scripts.tests.test_benchmark_visibility passes all23 cases. These include real
private Xvfb visibility/focus/occlusion, owned startup/error cleanup, exact
newline publication, bounded malformed records, held-child timeout diagnostics
and explicit handling of an exited original PID. Installed tool versions and
full result retained. No normal readiness/visibility assertion was skipped or
deadline increased. The existing diagnostic regressions remain permanent.

This confirms the current diagnostic/reproduction environment, not the old
cause. No equivalent deterministic Xvfb causal failure is established locally;
a deliberately sleeping substitute checks diagnostics, not Xvfb attribution.
T644 remains unresolved in Other, blocked on a recurrence retaining original
child/syscall/publication evidence or an equivalent causal reproduction. A
fix still requires its own permanent failing-before/passing-after regression.
It is not removed or presented as implemented. No network request, runner
workflow, live desktop, EVDI or service action occurred. The tests use only
owned private Xvfb children; existing cleanup terminates and waits on those
original process objects before their descriptors/tempfiles are released.
