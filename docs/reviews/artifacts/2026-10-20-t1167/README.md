# T1167 — shared private-connection access

Connection and saved-trust maintenance share one process-owned admission slot. Hosted cancellation and Binder tests retain that slot until actual delegate retirement; mode labels and logical completion cannot release it.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

One application-process ConnectionAccess now gates connection execution and the
saved-trust operation composed under T1168. A fresh reference-bound permit owns
one slot; mode observations carry no release authority. ConnectionBinding guards
the existing complete LifecycleRun, so logical end, cancellation, presentation
loss and retirement deadlines cannot admit a second native connection owner.
Constructing or retiring an unexecuted run must remain native-inert.

ResourceLane serializes original execute/retire and joins logical execution before
retirement. Its delegate must supply actual original private-worker/store and
completion-relay quiescence before successful retire. Failure retains the permit;
a retry may release only after that original delegate actually retires. No timer,
Ended message, framework disconnect, GC or reference count proves native release.
Unknown cleanup stays funded by the original lane/worker until actual settlement
or original application-process loss. This gate adds no thread, timer, native
handle or observer scope; its capacity is one and token identities never wrap.
Conditional eventual cleanup assumes original native operations eventually return
or their original process dies, with the existing worker joining its IPC jobs.

ConnectionMediaConsumerOwner.retire settles its completion relay, not separately
owned native camera/audio claims. Those claims retain their own original cleanup
sponsors and cannot retarget a new private worker. The permanent three-resource
trace checks late and duplicate receipts, cutoff and equal-valued replacement
leases during saved-trust ownership. This is a domain authority test; actual
camera/audio quiescence is neither inferred nor claimed by this gate.

Permanent tests cover bounded competing claims, stale/forged/idempotent release,
mode metadata, unstarted rejection, cancellation, owner-scope loss, failed
retirement, connection presentation loss, original Binder death and an original
blocked STOP IPC. Retirement timeout preserves admission until actual original
IPC return and join. All fixture threads/jobs and callbacks have original cleanup
sponsors; executor tests await actual termination. These are bounded contract,
fault and native Binder traces, not unrestricted interleaving or hardware proof.

All 1059 connection JVM cases pass with zero failures, errors or skips.
JaCoCo 0.8.15 passes 47/47 scoped production functions individually at least 80%,
including every mapped native variant. Four architecture checks pass. Offline
lint (zero errors, nine existing warnings) and APK assembly pass. Cached Kotlin
1.9.20 PSI measured 150 production/test entries, maximum 5; this is source audit,
not native SonarQube evidence (T1019). Binder/Robolectric and domain traces do
not qualify physical devices/providers (T863). No live app/device, installation,
network transfer or push occurred. Raw evidence remains at
`/backups/disk2/blent-work/2026-10-20-open-items/t1167`.

</details>
