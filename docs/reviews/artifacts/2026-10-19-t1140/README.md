# T1140 broker registration boundary

Broker registration composes the existing portable request while keeping Linux descriptors at the adapter boundary. Native server tests passed without changing admission or rollback behavior.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Reuse the existing portable registration_client::Request for the owned peer,
resource, credential and profile. Client/request correlation and owned Linux
descriptors remain explicit separate inputs. No public contract depends on the
Linux Server, route store or OwnedFd type; no second mutable owner is introduced.
Current production and native ambiguity-fixture callers move that same request.

Capacity/owner-ID checks still precede endpoint reservation. The existing route
owner and endpoints retain responsibility on success, rejected pairing, malformed
handles, failed link enqueue, cancellation and final route retirement. Rollback
still revokes the exact original claim and retires its endpoints; no new heap
allocation or escaping borrow is introduced by the composed record. This is a
behavior-preserving signature change, not a new transport/support claim.

All33 normal server fixtures pass, including invalid descriptors, ambiguous
certificate peers, private IPC/TLS and independent media retirement. Package-
scoped Clippy clears the argument-count diagnostic; three unrelated link/authority
size diagnostics remain. Source audit is <=9 (not actual SonarQube; T1019).
Cargo formatting passes. Source/compiled-object-attested native results follow.
No Windows/EVDI edit, physical USB/media action, install or network transfer.

Fresh source/compiled-object-attested LLVM collection passes33 server tests.
Every production function in routes.rs and commands.rs passes individually:
16/16 >=80% executable lines, no missing counters or new exclusions.
Raw evidence: `/backups/disk2/blent-work/2026-10-19-open-items/t1140/native`.
Pinned source audit checks20 production/test functions, maximum6.

</details>
