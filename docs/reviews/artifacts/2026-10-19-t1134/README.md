# T1134 bounded link admission payload

A boxed link-admission command preserves the exact original connection on full or closed queues. Real private-TLS ownership tests passed; no runtime speedup was claimed.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Only Command::Admit owns a boxed Connection. The link receiver consumes that
container back into its original local Connection before running media; no new
native owner, resource clone or replacement authority is created. The existing
bounded eight-command channel and admitted native resource budgets remain.
Rejected full/closed submissions return the same owned container to the caller.
Queued cancellation/drop destroys that same connection through its existing
owner; native completion remains separate from task termination or logical error.
The box adds one container per already admitted connection, with the existing
process allocator semantics; no new global queue or retention cycle is added.

A permanent normal-suite test checks full and closed queues with real private
TLS connection setup. It confirms the original native proof stays incomplete
through rejection and settles only after the returned connection is retired.
Every path sponsors task join and original cleanup before asserting, including
faulty admission. A baseline passes before boxing. A deliberate mutation drops
the original and returns a different command; the test fails on that mutation.
Final construction sites add Box::new; the ownership assertions are unchanged.
Both pre-box fault-stage and final oracle hashes are retained.

All230 normal broker fixtures pass after the change. Package-scoped host Clippy
clears the large enum and link submit diagnostics; only independent T1141 remains.
The pinned source audit stays <=9 (not actual SonarQube; T1019 remains).
No runtime speed claim, native USB qualification, live-device action,
Windows/EVDI edit, installation or network transfer. Joined lifecycle/native
process-loss qualification remains T862; fresh native counter results follow.

Fresh source/compiled-object-attested native collection passes230 broker tests.
All18 production functions in link.rs and server/events.rs meet >=80% executable
lines individually, none unmeasured, no exclusions. Pinned source audit checks36
production/test functions, maximum8; Cargo formatting passes. Raw evidence:
`/backups/disk2/blent-work/2026-10-19-open-items/t1134/native`.

</details>
