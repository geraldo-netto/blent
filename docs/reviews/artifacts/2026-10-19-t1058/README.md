# T1058: joined display wake acceptance

Joined wake tests kept camera, microphone and speaker runs independent while display wake paused and resumed. The focused 114-test collection passed; it measured neither physical media delivery nor battery savings.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

New normal-suite SDK27/34 acceptance composes SessionCoordinator's actual original
control source, StreamingPowerBinding/runtime and WindowPowerBinding with three
independent actual camera/microphone/speaker bindings. Eight display pause/resume
cycles, battery preference changes and foreground withdrawal retire display wake
without canceling or replacing any auxiliary original run. An independent held
microphone retirement leaves camera/speaker authority intact. Cleanup opens all
original gates and joins actual fixture jobs; each admitted auxiliary run settles
exactly once. This uses bounded simulated native runs and existing legacy
invitations, without claiming physical media delivery or USB consent acceptance.

A second joined case leaves original native admission queued across control
replacement. The old socket/view cannot reacquire. After a current resume queues
a native notice, malformed negative revision closes that actual source before
receipt; its retained permit denies admission independently of UI delivery. No
native wake worker opens, screen demand clears, supported-source history remains
and settings cannot override loss. This is desired-authority cutoff, not a native
quiescence claim. Original native retirement retains its separate runtime sponsor.

All four new SDK cases pass. The joined 114-test collection also passes existing
source/lifecycle, Window cancellation/write/reporter/reentrant faults, foreground
observer failures, late creators, held actual native retirement, source-permit
relevance, native worker/fault adapters and independent bounded request/wake models.
Normal/unsupported fallback and the full 72-policy table remain covered. This
acceptance composes those retained checks rather than equating child completion
with parent completion. No assertions, deadlines or exclusions changed.

Source hashes are frozen and match collection. Production sources exactly match
the T1061 native collection: its 39 changed functions/callbacks each have100%
native line coverage; no new production function is introduced here. This is a
focused 114-test pass, not a new complete Android or physical power measurement.
Kotlin PSI1.9.20 audit covers88 test functions/callbacks, maximum estimate5; native
SonarQube remains T1019. The two acceptance fixtures retain bounded original
resources through cleanup and join before their scope lifetime ends. No network,
installation, hardware interruption or battery-saving claim. Final display
acceptance remains T1011; physical host/tablet wake acceptance remains T914.

</details>
