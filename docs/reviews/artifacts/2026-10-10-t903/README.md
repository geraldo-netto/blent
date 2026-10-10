# T903 — exact host command outcomes

Submission returns a lifetime-local, ordered ID only after bounded queue admission.
A short submission mutex serializes concurrent producers; IDs stop at MAX_COUNTER
and are not consumed by full/closed queue rejection. No wrap or replay occurs.
The latest exact command outcome is published atomically with its complete view
or rejection and survives automatic refresh, close and terminal failure. Callers
needing every outcome serialize submissions. Missing outcomes never mean success;
timeout, protocol failure and cancellation leave unfinished commands unconfirmed.

The permanent queued-revoke regression failed before the fix: an unrelated
refresh had no separate command result and later polling erased rejection.
Additional regressions cover successful matching completion, stale actions with
no native send, concurrent producers, admission bounds, exhausted IDs, bounded
errors, timeout and cancellation. The pre-existing late-publication/retirement
checks remain. All 75 native Linux broker tests pass; all 23 scoped production
functions meet 80% individual executable-line coverage. All 56 scoped production
and test functions have cyclomatic complexity at most 9. Native counters, source
hashes, red/green logs and gate output are retained here. No live device test,
installation or download occurred; GUI controls remain T894.
