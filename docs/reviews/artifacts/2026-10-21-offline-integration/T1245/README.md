# T1245 rendered audio controls through private Binder and real sockets

Three permanent normal-suite scenarios run on Robolectric API27 and34. They join
the production audio controls, original consent policies, connection/media Binder
protocols, actor/multiplexer, real local endpoints, audio bootstrap and speaker PCM
renderer. Existing independent grant/PCM builders are reused. The new UI adapter
marshals synchronous control calls onto the fixture's original serialized owner
dispatcher; it duplicates no policy and creates no retained worker or queue.

- UI microphone Start obtains a synthetic original SDK reply before admission.
  Its real audio bootstrap reaches the private connection. A held original native
  read survives logical Stop and native-release permission; no terminal proof is
  sent until that read actually returns and the original worker/Binder retires.
  Speaker rendering advances while the microphone remains held; original display
  and pairing remain current. Each speaker/microphone terminal uses its own lease.
- Fresh speaker generation requires a new UI Start. Old rendered Start/Stop actions
  send no new consent commands, both before admission and during actual replacement
  playback. Original worker retirement precedes the replacement factory and new
  channel readiness is checked exactly. Replacement playback traverses its own
  grant, real socket and PCM renderer before its original terminal is accepted.
- UI microphone denial produces the exact management PERMISSION_DENIED result
  without opening a microphone worker. Original speaker rendering advances and
  display/pairing remain independent; speaker Stop retains its own cleanup proof.

The six cases pass after prerequisite corrections T1246 (arbitrary fixture clock
cap), T1247 (cross-owner rendered actions) and T1248 (historical Prepared shortcut).
The broader current offline cohort and release-build evidence are retained under
T1247; focused normal-suite XML and source hashes accompany this document.

## Resource ownership and scope

Each scenario owns one private connection/dispatcher, one display and at most
one microphone plus an original and replacement speaker fixture. Original audio
owners have separate scopes, sockets, claims and retirement gates. The native
read and cleanup gates are distinct: permitting cleanup cannot finish an active
held read. Original terminal publication is checked after its exact cleanup and
Binder unbind, not inferred from disabled controls or a timeout. Render callbacks
borrow their original controls only within the scenario; synchronous dispatcher
calls settle before returning.

Teardown closes microphone and speaker owners in nested `finally` blocks, then
their shared connection. Existing fixture cleanup releases original gates,
settles controlled workers and destroys its private service controllers. No live
native device is acquired. SDK permission, media devices and process death are
explicit synthetic boundaries; real Android process isolation/driver retirement,
acoustic quality and physical USB behavior are not established. Fair scheduling
and actual original native completion remain eventual-cleanup assumptions.

Speaker PCM is fixed synthetic data; no physical microphone audio is captured or
stored. The held microphone case observes bootstrap/read entry, not delivered
microphone PCM. Per-function production counters, source complexity estimates
remain explicit under T1247. Owner clarification2026-10-21 accepts local
cyclomatic tools; T1244 verifies these measurements and closes the former
native-analyzer prerequisite.
All execution uses cached tools in user/network namespaces with loopback only.
No physical check, service installation, upload, download or push occurs; live
acceptance and T1238 remain unfinished and paused.
