# T1241 original control failure cannot stop a replacement

A camera/audio action captured its original attempt, but `send` handled a later
rejection/exception with an unconditional `lane.stop()`. If reconnect had selected
a newer attempt, this stopped its active worker or discarded its queued request.
The permanent regression reproduces both outcomes for camera, microphone and
speakers, with returned rejection and thrown failure: all12 cases fail before
repair (`red.xml.gz`). Each fixture keeps original retirement behind an explicit
gate; equal replacement ticket values confer no original authority.

Failure cleanup now serializes with replacement selection using the existing
public-action monitor. Original media invalidation, target clearing and ended
publication retain their original identities. The lane cutoff checks the current
attempt immediately before Stop, after publication callbacks can select a new
attempt reentrantly. Media command delivery adds no new locking.

An initial entry-only identity guard still discarded reentrant replacements.
Another12 permanent cases demonstrate that counterexample with the same original
retirement gates. `reentrant-valid-red.xml.gz` records12 earlier cases passing and
12 observer cases failing; no fixture-coroutine errors are hidden. All24 cases
then pass with the final cutoff. The observer selects inside its callback and
observes actual fixture retirement afterward, rather than assuming synchronous
Unconfined-job completion. Existing current-command rejection/exception cleanup
remains covered by `ConnectionBindingTest`.

The broader normal Android cohort passes661 cases, with no failures, errors or
skips; the final focused cohort adds the bounded-ordering test and passes34.
Both use cached providers and Gradle offline inside an isolated network namespace.
JaCoCo0.8.15 native method counters measure `send`7/7 and `controlFailed`5/5 lines,
individually100%. The inlined check body belongs to `send`. Native report, exact
source/class hashes, red/green records and per-function counters remain here.
Kotlin PSI1.9.20 source cyclomatic estimates remain at most4 for changed/new
functions and callbacks. These are not licensed SonarQube measurements; native
SonarQube remains unavailable offline. No exclusion or test weakening is added.

Ownership: one captured Attempt is borrowed strongly until its command returns.
The helper allocates no native handle, coroutine, queue or retained callback.
The existing lane owns the original worker until actual retirement, with one
latest pending replacement. On current failure it requests original cancellation;
on stale failure its original metadata cleanup cannot stop the newer lane.
Concurrent replacement selection shares the cutoff monitor; reentrant selection
is checked after publications. Newer queued admission still waits for original
retirement. Late media claims remain bound to their original owner and relay;
unknown obligations keep their original independent retirement sponsors.

The fixtures establish bounded domain/controller behavior, not physical USB or
unrestricted leak freedom. Cleanup requires eventual original native completion
and fair execution of retained cleanup/relay jobs. No tablet action, app install,
service restart, package transfer or repository push occurred. T1238's earlier
physical Stop report remains uninstrumented and paused; this race is a confirmed
software defect, not a claimed hardware root cause.
