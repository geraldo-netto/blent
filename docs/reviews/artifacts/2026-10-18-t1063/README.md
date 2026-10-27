# T1063: original wake-handle ownership

A wake lease installs cleanup ownership before acquisition and retains uncertain handles until actual retirement. Fault tests and 1,296 model histories passed; native fence evidence remains a separate backend obligation.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

WakeHandle defines fresh unadmitted allocation, acquisition uncertainty and actual
original retirement independently of OS/framework implementations. WakeLease
installs the cleanup owner before acquisition, retains that exact handle/failure
through ordinary/fatal faults and rejects uncertain reacquisition or reentrant
mutation. Only normal original retirement permits a fresh successor. Settlement
also requires the factory/effect operation to have ended. No desired-off flag,
timeout, held diagnostic, reference count or cancellation request supplies proof.

The [contract](../../../display-wake-policy.md#original-wake-handle-ownership)
records bounded state, factory/partial-failure sponsors, callback/borrow cutoff,
transition preservation, no-premature-reuse invariants and conditional eventual
cleanup assumptions. The native backend must supply actual fence evidence; the
containing execution owner isolates and retains blocking calls until they settle.
This pure owner introduces no OS type, native implementation, timer or thread.
Original service/backend integration remains T1060, Activity integration T1061,
joined acceptance T1058/T1011 and physical host/tablet evidence T914.

Permanent tests cover admission before acquisition, partial acquisition, factory
rejection, repeated desire, fresh successors, release failure, fatal effects and
reentrant factory/native callbacks. An independent model checks all 1,296 four-step
fault/desired-state histories, including failure after a simulated grant. It checks
native-model conservation and original ownership after each step and retains an
independent handle list for unconditional fixture cleanup. The deliberate
early-release control fails both original-retirement and fatal-error regressions.
The permanent in-flight factory settlement regression was separately observed
failing before the view was fixed. Raw controls/red logs and final green evidence
are retained. These are bounded contract/model checks, not native hardware proof.

Fresh offline ordinary suite: **2,052 tests, zero failures/errors/skips**. All 13
authored production methods independently measure **100%** native JaCoCo 0.8.15
line coverage with pinned Kotlin PSI 1.9.20 inventory/inline attribution and no new
exclusions. Source/vector hashes, native class hashes, raw execution/XML counters
and individual reports are saved. Cached source audit covers 68 production/test
functions, maximum 8; this is an estimate, not native SonarQube certification
(T1019). Twelve architecture/contract/portability checks pass. No network transfer,
installation, push or live host/tablet interruption occurred.

</details>
