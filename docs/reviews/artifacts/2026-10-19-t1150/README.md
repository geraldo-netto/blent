# T1150 — saved-computer trust contract

Saved-computer views bind selections to exact identity and revision without exposing mutable trust storage. Contract tests passed; private-process maintenance and UI integration were separate steps.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The production display/webcam software adapters are integrated; their remaining
physical acceptance is separately deferred under T855/T858. T897 now proceeds as
an independent follow-up, decomposed into this domain contract, T1151 private
maintenance and T1152 presentation. This prerequisite alone does not expose an
offline UI, open USB, create a connection or grant media permission.

SavedComputers is a detached immutable presentation value with at most 1024
canonical, strictly ordered unique peers. Revision zero represents an unpublished
empty store. Selection binds the displayed tablet identity, positive persisted
revision and peer. The native adapter must also bind requests/results to the
exact original client/worker generation and never replay them after reconnect.
ConnectionTrust remains the sole mutable trust/revision owner. Snapshot reads
require a ready owner. Conditional forget validates identity/revision before any
pairing mutation; an absent peer changes nothing. Accepted forgetting reuses the
existing durable revoke path, including immediate pairing withdrawal and poisoned
state on storage failure. Close and poison cannot expose a successful snapshot
or forget; revision exhaustion never wraps or claims persistence.

Presentation data owns only bounded immutable values and copied lists. No native
resource, callback, allocator lifetime or storage ownership escapes in a view.
Callers serialize access to ConnectionTrust, which retains the existing exclusive
store until close; native execution/retirement remain adapter responsibilities.
Stale selection rejection preserves the current pairing and durable record.
Failure after accepted revocation keeps authority withdrawn while surfacing the
failed durable operation. Native cleanup fairness/provider limits are unchanged.

20 normal JVM tests pass without failures/errors/skips, including existing trust
and authenticated-admission regressions. New permanent tests cover empty/no-write,
restored state, detached lists, canonical ordering/capacity, local/revision guards,
absent peers, selected-pair withdrawal, unaffected other pairing, closed/poisoned
state, storage failure and revision exhaustion. Bounded properties reject 1024
stale revisions and 1024 absent peers; constructor cases include signed Long
extremes and capacities 0/1/64/1024/1025. This is a new planned contract, not a
change to the semantics of existing revoke callers.

Current-source JaCoCo 0.8.15 counters pass all 15 scoped production functions and
all mapped native method variants individually at least 80%. The accepted report
was generated only after the instrumented Gradle task completed; an earlier
premature check of the previous report is retained outside the accepted artifacts.
Compiled-class hashes, source manifest, raw exec/XML and test XML are retained.
The cached Kotlin 1.9.20 PSI audit covers 53 production/test entries, maximum 4;
this is a source estimate, not native SonarQube evidence (T1019 remains blocked).
Android contract/lifecycle architecture checks pass. All dependencies and hosted
API27/34 jars were already cached and checked locally; Gradle and Robolectric
were forced offline. No install, live device/service action, transfer or push.
Raw evidence: `/backups/disk2/blent-work/2026-10-19-open-items/t1150`.

</details>
