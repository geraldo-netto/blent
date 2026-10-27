# T1263: preserve newer observations during cancelled-drain cleanup

Cancelled UI drains preserve newer pending observations under the same owner lock. Deterministic lost-update regressions and the full hosted suite passed; the result does not attribute every historical timeout.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

A deterministic normal-suite regression now forces the lost-update boundary:
queue the original drain, cancel its job, submit a newer observation, then execute
the cancelled continuation. The old finish(token) erased the replacement and
left observed=-1 instead of 0. The regression fails before the fix and passes
for bursts of 1, 2 and 1000 replacements afterwards. Separate clear/scope-stop
cases preserve cutoff. Existing status, rejected UI dispatch and ordered Activity
regressions remain unchanged in their assertions and deadlines.

LatestDispatch owns one pending observation and an identity-only admission token.
A cancelled drain discards its own observation but hands a newer pending identity
to a fresh drain under the same lock. It never resubmits an old snapshot over a
concurrent newer submit. Tokens retain no view/action; clear drops the pending
borrow, scope cancellation prevents a replacement, and retired UI epochs retain
their existing independent identity guard. Success, failure, cancellation and
reporting exceptions all settle through the same finish boundary. Conditional
progress requires eventual execution of dispatched continuations. The finite
forced schedules do not prove every possible native/UI execution.

The full offline Android suite passes all 2710 tests with the concurrent T1262
format fix. The original 69-class SavedUiOrderTest prelude and both ordered API
27/34 Activity tests pass; ordered-ui.xml.gz preserves that result. The deterministic
counterexample establishes a production lost-update bug consistent with the
review failure; it does not prove that every prior intermittent timeout had only
this cause. No global log reset, skipped test or relaxed timeout was used.

All 13 inventoried LatestDispatch functions/initializers/lambdas have 100% native
JaCoCo 0.8.15 method-line coverage. Kotlin PSI 1.9.20/local cyclomatic measurements
are <=4 across the changed production/test files. The raw shared full-suite report
is ../2026-10-26-t1262/jacoco.xml.gz. Source hashes and both failure/pass logs are
retained. Physical Android evidence remains outside this hosted JVM verification.

</details>
