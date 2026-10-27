# T1056: original control-scoped activity publication

Original authenticated control sessions now publish bounded activity hints and preserve coalesced Stop. Private-socket tests do not establish physical rendering or Android wake behavior.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The [independent signal contract](../../../display-activity-signal.md) is now
published by original host input/control sessions. Portable session startup binds
its existing source before listeners/work exposure. No source factory is added.
Configured observation appears in the authenticated initial greeting and bounded
standalone updates. Pending/failure/loss reports inactive; absent observation omits
the protocol. Coalesced stops retain their fence and unchanged observations emit
nothing. Source notices arm before reads, including changes during greeting/write;
closed channels cannot spin. Current ownership/input event ordering is preserved.

Six permanent tests cover pre-poll publication, coalesced stop/resume, absence,
closure/no spin, canonical bounded encoding, actual portable-session initial
handoff, wrong authentication with configured observation, original controller
replacement and exact attachment cutoff. Existing protocol/native input tests
remain intact. The moved notice implementation has one authoritative shared body.
Socket/task/attachment owners and existing cleanup sponsors remain authoritative;
read views retain no source creation, media grant or native release authority.
Each server admits at most 16 sockets; each retains one projection, one latest
source view and one bounded serialized response/notice, with no detached task or
hint queue. Common T1055 tests cover finite counter exhaustion; propagation ends
that original handler. No native full-counter-cycle experiment is claimed.

Fresh full CLI collection passes 835 library and 545 daemon tests, with three/four
existing ignores. All 16 scoped production functions independently measure >=87.5%
executable-line coverage. The initial broader SDK input attempt passes 93 tests but
fails the unchanged T604 private-bus fixture because `dbus-run-session` is absent;
the failure is retained under existing blocked T835. No test/threshold was removed
or weakened, and no broad SDK pass is claimed. Fresh scoped network-disabled SDK
25.08 collection passes three source-view, 12 runtime-work, six activity, seven
controller-contract, one idle-wakeup and 46 input protocol cases. All 16 functions
independently measure >=84% there. Source, Cargo/fixture inputs, object hashes,
original LCOV, inventories and per-function reports are retained separately.

Repository source audit covers all 35 scoped production/test functions, maximum
cyclomatic score 9, using cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2. Native
SonarQube remains T1019. Twelve architecture/contract/portability checks and the
production accessory build pass. Existing controller epoch wrapping was added to
matching T865 rather than duplicated or silently changed. Native Windows runtime
work remains skipped; defaults do not announce an absent activity source.

Android decoder/wake consumption remains T1057/T1058 and acceptance T1011/T914.
No network transfer, installation, upload, push, live lock/suspend/display change
or tablet work occurred. Hosted private sockets establish this bounded transport
composition, not unrestricted kernel/device safety or physical rendering.

</details>
