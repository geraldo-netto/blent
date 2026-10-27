# T1069: current-demand permits through native admission

Native admission rechecks the exact display-source view, even after delayed creation or reentrant callbacks. The final clean collection passed after separately recorded stale-compiler and inline-counter gaps were addressed.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Independent immutable demands/requests carry a read-only original-view permit.
Fixed policies retain a fixed permit plus original foreground identity. Logical
channel relevance stays exact latest-request identity; actual receiver and lazy
worker admission additionally validate the desired source view. A changed view
with identical power still publishes. Receipt settles only its original notice;
fenced callback retry preserves the actual current permit. Withdrawals never read
a vanished source. Reader exceptions fail closed; fatal identity is preserved.

The receiver rechecks foreground relevance after reader calls. A deliberately
reentrant reader cannot install a replaced owner's dormant workflow. The lazy
wrapper waits for actual current delivery, installs cleanup ownership before the
creator's result is used, then checks current relevance again before execution.
A late creator stays owned while awaiting a fresh view; original actual retirement
and source relevance remain independent gates. Same native tasks refresh their
view without duplicating the original process. Idle timers also check current
source relevance; readiness remains actual original-task acquisition information.
The contract records bounded references, cutoff, handoff and eventual-progress
assumptions; no new native allocator, queue or timer is added.

Permanent regressions failed before their fixes: stale queued receiver admission
(both SDKs), invalid view before lazy creation, view invalidated during held creation,
permit loss through callback retry (both SDKs), and reentrant-reader stale workflow
(both SDKs). Tests retain original cleanup sponsorship, fresh same-policy views,
1,000 coalesced publications, stale idle timers, ordinary/fatal reader faults and
independent native-retirement/source gates. Existing binding/cleanup regressions
keep all prior assertions while using typed demands.

Final **clean offline** ordinary suite: **2,179 tests, zero failures/errors/skips**.
All **31** maintained authored production functions/callbacks independently measure
**100%** native JaCoCo 0.8.15 line coverage. Both producer callbacks have explicit
SessionCoordinator.kt method counters; their raw counters are retained. Frozen
source/vector and compiled class hashes, native XML/execution data and pinned
Kotlin PSI 1.9.20 inventories are saved. Source audit covers **320** production/test
functions, maximum estimate **8**; native SonarQube remains unavailable (T1019).
Twelve architecture/contract/portability checks pass; no new exclusions.

Nonpassing preliminary evidence remains: 12 stale incremental test callers invoked
an obsolete internal signature (T1071); clean rebuild recovered them without ABI
shims. An inline Flow.map producer exposed missing semantic-body counters despite
a covered factory (T1074); the current producer replaces that wrapper and measures
its owned callbacks directly. Earlier counter-gapped data is not an accepted
coverage pass. Clean compilation also exposed 32 existing fixture warnings (T1072).
Normal policy-observer failure containment remains T1073. Current Activity producer
still requests Waiting/fixed permits until T1061 source/window integration; joined
acceptance remains T1058/T1011 and physical host/tablet evidence T914. No network,
installation, push or live device interruption occurred.

</details>
