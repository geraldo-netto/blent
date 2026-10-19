# T1093: original transport budgets as immutable snapshots

ConnectionDeadline and ConnectionDeadlineSource define an independent portable
metadata contract. A snapshot reports the earliest known absolute elapsed-realtime
budget and whether every contributing provider exposes its budgets. Missing
providers remain explicitly unknown. Composition retains a tighter known deadline
alongside unknown providers, so future actor waits can preserve both that budget
and the required legacy fallback. Null means no budget in that owner, not native
retirement. Nonnegative Long values, including Long.MAX_VALUE, are representable;
no numeric sentinel is used for snapshot absence.

Existing frame clocks, readers, writers, fixed four-slot resource admission,
TabletSession and ConnectionSessionRunner project their authoritative state.
Partial progress/flush keep the original frame budget; idle reads have none.
Admitted/closing resources expose no further admission budget. Runner stop reports
its original retirement deadline; an already acted-on termination requirement or
actual retirement needs no additional protocol deadline. Native obligations still
remain independently owned until their original completion evidence settles.
No actor consumes these projections yet. Timer frequency and all runtime deadline,
authority, I/O, cancellation and native retirement behavior remain unchanged.
T1094/T1095/T1092/T918 retain native setup/local aggregation, actor waits and matched
performance acceptance. This feature alone claims no polling or CPU reduction.

Ten permanent normal-suite cases cover independent sorted composition oracles
across all 1,728 triples from twelve bounded snapshot states, commutative/grouping
checks, 128 seeded negative values plus boundary negatives, MAX/zero values, original
clock failure/clear/cancel, partial reads, cancelled outstanding loans, partial
writes/flush, resource admission/expiry/revocation, tablet projection and mixed
known/unknown runner/retirement budgets. Before capability projections were
implemented, all seven transport cases fail against explicit unknown projections;
the policy cases already pass. All ten pass after implementation. A deliberate
control hiding another provider's unknown status fails exactly three permanent
cases, including the independent oracle. Its original source is restored and
verified before the final clean collection. Raw red/green/control logs remain.

The contract owns only immutable nullable time and Boolean values. Projections
borrow existing state synchronously on its serialized owner and return no mutable
view, native handle, resource lease or retained provider. Composition stores no
source callback. Work is fixed-size except the resource scan, bounded to four slots;
there are no new native admissions, tasks, queues, reclamation rules or release
paths. Diagnostic idle/expiry never returns native buffers or admits replacement.
Native cleanup remains conditional on the existing exact original workers/handles
actually retiring; metadata neither proves leak freedom nor changes stuck-owner
funding. Tests use bounded in-memory native-I/O substitutes, while retained related
loopback regressions exercise actual JVM socket worker cleanup. No physical Android,
USB, media device or suspend/resume acceptance follows from these hosted checks.

A fresh clean offline native JaCoCo collection passes all 78 selected new/related
framing, resource, tablet, runner and loopback cases, with no failures/errors/skips.
The strict current coverage checker measures all 17 changed production functions
and authored initializers independently at 100%. Raw native exec/XML, compiled
class hashes and frozen Kotlin source hashes are retained. This is focused
validation, not a repeat of T1091's full 2,237-case suite. All 12 architecture checks
pass. Pinned cached Kotlin PSI 1.9.20 audits 175 production/test functions/callbacks,
maximum repository Sonar-style source estimate 8. Native SonarQube remains
unavailable under T1019; these estimates are not native SonarQube evidence.

No network transfer, download, installation, restart, physical handling or push.
