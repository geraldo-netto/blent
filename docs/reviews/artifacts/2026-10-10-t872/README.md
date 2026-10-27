# T872 authenticated host resource runtime

The host resource runtime orders complete management publications, bounded channel loans and exact retirement callbacks. Tests cover draft race fixes and sibling progress, without claiming a complete production broker or tablet display.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Implemented the host Session owner over the T873 frame driver, management decoder
and T851 Resources state machine. Register/Prepared/Commit/Admitted/Revoke handling
is serialized with whole-record publication completions. Media CREDIT validation
is held for the committing generation; management credit continues. Early matching
Admitted/Revoked records have one bounded deferred slot per resource, and duplicate
or stale records fail the link. No queued command, partial write or cancelled wait
establishes publication. Failed framing drops the stream and disconnects resources.

Each installed media channel has a bounded byte ring and at most one consumer
borrow. Completion includes a nonwrapping borrow ID and exact byte count; receive
credit returns only after actual consumer output completion. Readiness uses five
fixed coalesced slots, avoids per-frame peer-string clones, and polls transport
before returning notifications so tiny reads cannot starve peer revocation. One
stalled consumer uses its own receive window without blocking other channels.

Resource release requires matching peer settlement, every channel's terminals,
writer retirement and an explicit callback after actual local bridge retirement.
Disconnect waives peer barriers only. The Connection wrapper rechecks current
Pairing against verified setup and retains native Completion separately through
failed helper retirement. A real isolated Linux helper fixture establishes child
exit before native completion; cancellation or a failed close is not success.

Production wire namespace is 1 for each fresh mutually authenticated TLS stream;
local pairing epochs remain independent. Fresh TLS keys and disabled resumption
prevent cross-stream byte replay, and callbacks carry full local resource leases.
Both peers must agree the symmetric channel_bytes setting over authenticated TLS
before CREDIT. T853/T854 own that setup exchange and periodic idle tick scheduling;
constructor parameters or local approval alone are not protocol negotiation.

Offline validation: 52 broker tests, 329 retained common tests and seven accessory
setup/native-owner tests pass. All 82 scoped production functions pass the native
per-function 80% LLVM gate; 174 reviewed production/test functions have complexity
at most 8. Tests include stock mutually pinned TLS with admitted duplex control,
fragmented writes, final-fragment pending flush with early CREDIT/Admitted/Revoked,
permission and rejection outcomes, independent consumers, actual helper exit,
stale callbacks, malformed/bounded inputs and generation reuse.

Retained fail-before-fix logs cover eleven initial-draft bugs: expiry double-close,
peer terminal retaining resource authority, pre-Commit terminal corrupting an
inactive generation, stale monotonic time across await, missing readiness for
pre-admission data, disconnect missing the final settlement, equal-length stale
consumer completion, missing remainder readiness, duplicate readiness accumulation,
notification starvation of peer revocation, and invalid receive-gate generations.
The separate epoch regression covers independently retried local pairing counters.
All permanent tests remain in the normal feature-enabled library suite.

T872 supplies runtime/library boundaries, not a production broker executable or
ADB-free display. T853 still owns election, client/bridge tasks, discovery,
authenticated window agreement and persistent pairing orchestration. T854 supplies
the Android connection service; T855 pairing UI and T857 rendered display/touch
acceptance remain. Webcam follows that first user-visible milestone. No downloads,
application installs, live service restarts or USB-mode changes were performed.

</details>
