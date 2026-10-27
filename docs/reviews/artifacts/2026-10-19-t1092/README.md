# T1092: bounded absolute-deadline actor waits

Connection actors wait for notifications or the nearest known deadline, with bounded fallback for unknown providers. Deterministic deadline tests and all 2,291 hosted cases passed; expiry never substitutes for native cleanup evidence.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Both connection actors now sample their original owners' independent deadline
capabilities and wait for a retained wake or the nearest absolute deadline, capped
by the configured fallback. Unknown providers retain the legacy 10 ms cap without
hiding a tighter known deadline. Fallback also respects the minimum configured
native/approval/transport/retirement/resource budget, preventing a lost asynchronous
tightening hint from hiding a short user setting. Retirement retains the legacy
10 ms observation cap for exact original worker/thread settlement. Due results
request another owner advance; they never establish native completion.

ConnectionWaitLimits is an independent immutable narrow scheduling view. Spec
projects configured limits once; the pure wait policy validates nonnegative clocks
and fallback bounds, preserves zero/MAX deadlines and subtracts only safe positive
remaining time. Main admission now requires an explicit clock. Android's entry
adapter supplies the same existing monotonic provider to native admission and its
actor; portable orchestration imports no Android clock. Every current caller is
updated, including explicitly controlled fixture clocks. No legacy constructor or
duplicate mutable clock/budget owner was introduced.

The default timer remains 10 ms at this revision. T918 retains matched idle CPU,
wakeup, first-change/allocation measurements and any resulting default tuning;
these changes alone claim no measured CPU or battery gain. Existing configurable
larger fallbacks now honor deadlines instead of blindly delaying their observation.
Public wire contracts, trust/media authority and native retirement predicates remain
unchanged. All replacement decisions still require original completion evidence.

Fourteen permanent normal-suite cases cover a 240-combination independent sorted
wait oracle, invalid boundary and seeded negative/oversized values, safe MAX/expired
arithmetic, all configured short-budget limits, known/unknown idle behavior, nearest
deadlines, 64 coalesced/spurious hints retaining original budgets, deliberately lost
tightening hints and stuck worker/permission retirement requiring original proof.
Ten actor cases fail against the original fixed polling while four pure-policy
cases pass; all fourteen pass with integration. Virtual time makes these deterministic
and independent of host timing. The existing bounded timer identity/cancellation
model and original native ownership/isolation regressions remain in the full suite.

Wait policy owns only immutable scalar values with fixed input/output bounds:
fallback 1..1000 ms, minimum configured budget 1..600000 ms, nonnegative Long clocks
and deadlines, and result 0..fallback. It retains no provider or native view. Actors
continue owning their bounded conflated wake/command channels, scopes and original
native resource owners. Timeout cancellation disposes only the original receive;
logical wake/expiry cannot release a resource or admit replacement. Original jobs
and native obligations remain retained through existing cleanup sponsors on success,
error/cancellation and exhaustion. Conditional eventual cleanup still requires
original workers/backend evidence and scheduling progress; no unrestricted native
safety/leak-freedom claim follows from hosted counters or managed references.

Fixtures hold one actor/job per scope with a shared injected virtual clock and
bounded metadata timers. They settle simulated original ownership, cancel/join the
original job/scope and drain timer registrations on failed assertions too. Deadline
providers are independent fake contract adapters; no USB/media hardware is opened.
Physical Android/process/device acceptance remains separately recorded in TODO.

A fresh clean full offline JaCoCo collection passes all 2,291 tests without failures,
errors or skips. All 17 changed production functions/constructors/callbacks measure
100% native line coverage individually under the current strict inline/method rules.
Raw exec/XML, frozen source hashes, native class hashes and per-function results
are retained. All 12 architecture checks pass. Cached pinned Kotlin PSI 1.9.20 audits
440 functions/callbacks across changed production/tests, maximum repository
Sonar-style estimate 8. Native SonarQube remains unavailable under T1019.

No network transfer, download, installation, restart, physical handling or push.

</details>
