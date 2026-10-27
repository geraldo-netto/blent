# T941 original camera publication and terminal IPC relay

The original camera cleanup relay remains usable after Stop and cannot bind a replacement. Exact publication identity prevents stale completion or permission actions from changing a newer owner.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Private service controls remain owned during actual worker retirement. Only
capture-consumer completion may pass the stopped gate; new consent is rejected.
Main-process client retains a separate bounded cleanup relay to the original
already-started Binder. It cannot bind/start a replacement. Post-stop delivery
waits for the original STOP call, while independent EXIT remains available.
Blocked cleanup IPC remains owned even after Binder death and prevents successful
retirement/unbind until that call actually exits. Queues cannot grow beyond their
bounds. Execution Ended and framework disconnect still do not prove process death.

Main admission retains one exposed camera consumer lease separately from camera
visibility. Closure revokes visibility immediately while retaining exact cleanup
through original source loss. Only that closed published claim may forward proof;
stale/foreign/unpublished/repeated cleanup cannot release a replacement. Native
stop is requested before synchronous closure callbacks may send completion.

Camera presentation uses exact original generation/reference identity. Pending
permission and admitted routes are distinct; same-counter choices in replacement
processes cannot authorize callbacks. Monotonic owner retirement rejects stale
closures and closed-owner replay. New owners may change profiles, while conflicting
profiles for the same retained owner are rejected. Camera closure preserves display;
stop/revoke/peer changes revoke camera immediately. No metadata is camera permission.

Permanent regressions fail before client/service/admission stop paths preserve
terminal control and before the prototype reducer accepts higher-owner profile
replacement. Admission is independently re-proven red with a backend fixture that
correctly implements T940 terminal controls, then the same tests pass. Additional
fixtures cover real Robolectric Binder/Parcel on API 27/34, exact process generations,
bounded backlogs, blocked native delivery after death, rejection/failure, presentation
replacement, and 128 stale-reference callbacks. A draft negative-boundary fixture
incorrectly expected rejected one-way service delivery to return normally; it now
checks process exit and zero consent dispatch. No product bug is claimed from that
fixture error.

All 699 offline connection tests pass. All 121 scoped methods meet individual
native JaCoCo 80% executable-line coverage, all 14 authored constructors pass
(supplementing T848), and all 182 reviewed functions have cyclomatic complexity
at most nine. Native counters, inventory, hashes and red/green logs are retained.

T936 joins private IPC with the real local camera channel. Production capabilities
remain display-only; T932 must compose Activity/isolated hardware capture and its
actual completion proof before enabling camera. Host adaptation stays T933 and
physical acceptance T858. These are local transport/lifecycle fixtures, not hardware
camera, real codec or physical native USB acceptance. No downloads/device changes.

</details>
