# T1168 — bounded saved-computer coordination

SavedTrustBinding coordinates one application-owned maintenance request through
ResourceLane and the same ConnectionAccess contract as connection execution.
T1169 supplies Android application composition and the settings UI; this commit
alone does not expose a production UI or establish physical acceptance.

One fresh client identity controls commands. Reattaching revokes every old
client's refresh/stop/select/forget actions, including confirmation of identical
peer/revision values. Closing a view revokes only its identity and never cancels
an accepted operation or a replacement view. A choice additionally binds the
exact immutable displayed snapshot and canonical local/revision/peer triple.
The existing exclusive private store checks revision again at mutation time.
A forged absent-peer selection originally threw from SavedComputers.select;
the permanent normal-suite regression failed before admission was corrected and
passes unchanged afterward. Its complete file is sealed with the red evidence.

A collector accepts at most 1024 peers in 22 pages of at most 48, with contiguous
offsets, consistent identity/revision/total and global strict ordering. No partial
result is published. An explicit complete empty/no-identity result is distinct
from no result. Every admitted total 0..1024 is exercised, as are gaps, replays,
header changes, overlapping boundaries, stale callbacks and incomplete results.

The caller supplies an application scope, a native-inert factory and the shared
access slot. The original ResourceLane retains execution and non-cancellable
retirement across view or scope loss. Ready requires complete logical output AND
successful original retirement; deadlines only show waiting, while unknown close
quarantines the lane and retains its permit. Native factories own partial failures
and must settle original store/process/IPC before successful retirement. A stopped
forget may already have committed, so cancellation requests refresh rather than
promising rollback. No identity generation, pairing or media consent is added.

Heap retention is bounded to one current client identity, one request/collector
and one complete snapshot; old view objects carry no current authority. The lane
and its retirement guard sponsor uncertain native obligations until original
settlement/process loss. No new independent worker/timer/queue is introduced;
existing lane and worker ownership remains authoritative. Cleanup liveness is
conditional on eventual original native return/close or original process-death
proof, not a timeout/reference count. Failed proof never permits reuse.

Permanent traces cover both connection-admission orders, unstarted cleanup,
view replacement, accepted forget through view loss, cancellation, factory/store
failure, missing/late results, deadline and unknown retirement. Joined tests use
the existing SavedTrustWorker, trust codec and conditional store adapter for
1024-peer list/forget, external revision conflict and byte-preserving write failure.
A synchronous store-close latch proves view loss/cancellation cannot release the
slot before actual original return. Fixtures release and join original work.
These bounded fake-store/domain traces complement the prior native Binder/store
suite; they do not certify unrestricted native safety or physical devices.

All 58 selected JVM cases pass without failures, errors or skips. JaCoCo 0.8.15
passes all 46 scoped production functions individually at least 80%, including
every mapped native variant. Four architecture checks, offline lint (zero errors,
nine existing warnings) and APK assembly pass. Cached Kotlin 1.9.20 PSI measures
138 production/test entries, maximum 6; native SonarQube remains unavailable
(T1019). No live app/device, installation, network transfer or push occurred.
Raw evidence: `/backups/disk2/blent-work/2026-10-20-open-items/t1168`.
