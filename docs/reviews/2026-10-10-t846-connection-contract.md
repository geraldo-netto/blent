# T846 connection contract and integration boundary

## Implemented domain layer

`common/src/connection/` and Android `com.blent.connection` implement the same
bounded envelope, duplex credit state, fair multiplexer, public identity discovery and pairing authority state machine. These are
portable domain components, not an enabled production connection. Existing
ADB routing and the installed applications are unchanged by this layer.

The native synthetic USB trial is recorded separately in
[the accessory feasibility review](2026-10-10-t846-usb-prototype.md).

### Envelope version 1

Every envelope has a fixed 40-byte big-endian header. Decode validates the whole
header before accepting a payload length; encode validates before changing output.
Unknown kinds/channels/versions, nonzero reserved fields, out-of-range counters
and inconsistent body fields are rejected. Header parsing provides **no peer
authentication, pairing approval, camera permission or microphone permission**.

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 4 | ASCII `BLNT` |
| 4 | 1 | Version `1` |
| 5 | 1 | Kind: DATA=0, CREDIT=1, CLOSE=2, RESET=3 |
| 6 | 1 | Channel: control=1, display=2, camera=3, microphone=4, speaker=5, management=6 |
| 7 | 1 | Reserved, must be zero |
| 8 | 8 | Session, 1 through signed 64-bit maximum |
| 16 | 8 | Channel generation, same range |
| 24 | 8 | Sequence, 0 through signed 64-bit maximum |
| 32 | 4 | Payload byte length |
| 36 | 4 | Granted credit bytes |

DATA has 1..65,536 payload bytes and zero credit. CREDIT has no payload and grants
1..1,048,576 bytes. CLOSE/RESET carry neither payload nor credit. The current flow
owner treats either terminal kind as immediate revocation of both channel directions.
Already granted peer traffic drains without delivery until the peer terminal.
Replacement requires both terminals, the writer and local consumer to have retired,
and a strictly newer generation agreed through authenticated management. Channel IDs describe roles, not
permissions or independently supported native backends.

### Credits, consumption and retirement

A flow starts with zero credit in each direction. Sending CREDIT reserves receive
capacity. Receiving DATA debits that credit and retains the received byte count;
arrival alone does not free capacity. Only consumer release permits regranting
those bytes. CREDIT overflow, unsolicited DATA, wrong session/channel/generation,
duplicate/out-of-order sequence and post-retirement traffic are rejected without
changing state. Counters never wrap. Capacity is configurable from 1 byte to
1 MiB per direction per channel; adapters must choose useful defaults and expose
user-facing capacity options with their tradeoffs when enabled.

A single connection owner serializes flow operations. It validates an incoming
header before reading its body into already reserved storage. A truncated body
retires that stream. It reserves bounded writer capacity **before** committing
`Flow.send`; cancellation is allowed before commitment. After sequence/credit
commitment, a failed or partial physical write retires the generation rather than
refunding bytes which the peer may already have received.

### Fair payload scheduling and ownership

Each channel has a FIFO with independently configurable byte capacity (up to 1 MiB)
and message capacity (up to 1,024). Limits cover both payload allocations and
message-count overhead. Rust charges the vector's allocation capacity until its
last slice is consumed; Android charges the entire array. Successful admission
transfers ownership. Rejection returns ownership unchanged. Android adapters must
not mutate an admitted array or retain a slice beyond its writer reservation.

Round-robin scheduling considers at most six channels, skips channels without
credit and returns a bounded slice, limited by credit, remaining bytes and a
configurable 1..65,536-byte quantum. It borrows the original buffer without a
staging copy. Only one writer reservation exists. Successful completion advances
it; unsent cancellation preserves bytes. Clearing its channel before writer
retirement is rejected. A blocked media consumer exhausts only its own credits;
other channels remain eligible. Physical USB failure still affects the shared
link and requires bounded native supervision, not a claim of independent cables.

The scheduler carries DATA only. The multiplexer reserves one coalesced credit
and terminal notification per channel independently of DATA credit. It alternates
metadata and DATA and rotates metadata across channels. Sequences are assigned at
reservation; metadata cannot reorder already committed payloads. Failed writes
poison the physical connection rather than retry potentially transmitted bytes.

## Discovery, approval and native ownership

The bounded eight-byte `BLPA` discovery header identifies version, endpoint role
and a 1..4096-byte public certificate. Discovery is untrusted. The pairing state
machine binds approval and subsequent TLS proof to an exact certificate fingerprint
and epoch; stale callbacks, deadline expiry and revoked leases cannot authorize
streams. Cancellation enters Closing, with no replacement until native retirement.
Persistent trust and production approval UI are still pending.

After local approval, each peer sends the eight-byte `BLPR` readiness marker
(version 1, endpoint role, two reserved zero bytes), then validates the peer's
marker before starting the ten-second TLS deadline. Readiness is untrusted
scheduling information, never authentication or permission. Android bounds it by
the original approval deadline; the host candidate bounds its wait to 120 seconds,
and its caller must recheck pairing authority after authentication. A permanent
regression models twelve seconds of approval skew without consuming the TLS budget.

Linux uses one supervised `blent-usb-io` process per physical USB path, with a
per-port ownership lock and two fixed 64 KiB buffers. `host/accessory/` holds the
native adapter extracted from the historical synthetic probe. The Rust owner
reuses `OwnedChild` retirement/quarantine; requesting termination is not retirement.
Its discovery/setup boundary bounds parsing and handshake before exposing a
verified stream to the authority owner.

Android uses a non-exportable P-256 AndroidKeyStore identity under a fail-fast
cross-process file lock. Keys permit SHA256 and NONE digest use because stock
Conscrypt signs already-hashed TLS inputs using NONEwithECDSA. Loading an identity
proves both signature modes; an incompatible persisted key fails before USB open,
without silent deletion or rotation. Existing malformed or partial identities fail
without silent rotation. Stock TLS receives its key through a focused key manager.
`AndroidAccessoryPorts` owns duplicated descriptors; `SocketBridge` verifies a
private loopback socket pair and pumps two bounded buffers for stock socket-based
TLS. Both sockets enable TCP_NODELAY to avoid small-record delays. Linux terminates
packet-aligned USB OUT chunks with an explicit zero-length transfer so Android's
larger gadget read can complete. Both changes have permanent failing-before/passing-after
regressions, including invalid packet sizes and socket-setting rollback. It retains retirement responsibility through startup/close failures and
blocked native I/O. Absolute native deadlines remain the process supervisor's job.
The profile-only encrypted probe composes these adapters. The Android 16 native
provider and duplex transfer pass the diagnostic trial below. Debugging-off synthetic qualification also passes (below); production lifecycle
integration remains open.

## Experimental authentication adapters

Opt-in host `accessory-transport` and Android `SecureConnection` use stock TLS
with mutual certificate authentication, exact approved leaf-certificate pins,
and an authenticated protocol preface; API 29+ also requires `blent-usb/1` ALPN. These adapters require an approved certificate from their
caller; parsing a certificate does not grant approval. Approval UI, persistent
trust, revocation and runtime wiring remain unimplemented. The optional Cargo
feature uses cached dependencies; all pre-existing locked package versions are
preserved. New cached crypto dependency notices and archive checksums are retained in
`licenses/accessory-transport/`, included by the existing distribution-docs list.

The Rust adapter bounds the whole handshake to ten seconds. Android's temporary
ten-second socket read timeout bounds individual handshake reads only; an isolated
worker supervisor must additionally enforce an absolute handshake/write/close
bound. Android restores the caller's read timeout after authentication. Permanent
T846 regression first reproduced the leaked handshake timeout, then passed for
both default zero and a nonzero timeout after the fix.

Current local tests exercise Rust-to-Rust, JVM-to-JVM and actual Rust-to-Android
JVM adapter interoperability, rejected peer pins, protocol mismatches, malformed
certificates and cleanup. API 27/28 uses the authenticated preface without public
ALPN calls. JVM coverage does not establish native Android provider behavior. The separate
Android 16 device trial below establishes that provider only; API 27/28 native
provider qualification remains unavailable.

## Validation

Shared `testdata/connection-v1.json` and `connection-flow-v1.json` are independent
cross-language byte and state-transition expectations. Permanent normal suites
also exercise invalid header extents/fields, 4,096 bounded header mutations,
1,024 duplex release/regrant rounds, 4,096 queue admission/cancellation rounds,
retained allocation limits, zero-copy identity, channel starvation prevention,
stale generations and sequence exhaustion. Tests do not equate CRCs or IDs with
cryptographic authentication.

Review found inconsistent handling of oversized scheduling credits: Rust reserved
a payload while Kotlin rejected the input. A permanent Rust regression reproduced
that behavior before the fix. Rust now reports an explicit error before mutating
writer state, matching Android rejection. Both reject credits above 1 MiB.

Per-function native LLVM/JaCoCo evidence is collected for this new scope. T848
tracks an existing inventory gap for authored Kotlin primary constructor/init
bodies; their JaCoCo constructor counters are additionally checked directly.
A scoped pass is not a whole-project or physical-backend coverage claim.

## Native encrypted diagnostic trial

On October 10, the Linux host and Android 16 tablet completed 4,096 mutually
TLS-authenticated synthetic echoes across six channels: 58,803,628 payload bytes
per direction in 16.158 seconds. Both reported success; the tablet additionally
confirmed the host's final receipt, drained TLS close notification and actual
native bridge thread retirement. USB identity was `18d1:2d01`: debugging remained
enabled for direct diagnostics. This is not debugging-off acceptance or production
media qualification. Raw journals and source hashes are retained with the
[connection evidence](artifacts/2026-10-10-t846/connection/README.md).

| Payload bytes | Samples | Median echo RTT (ms) | p95 RTT (ms) |
| --- | --- | --- | --- |
| 1 | 684 | 1.851 | 2.838 |
| 64 | 684 | 1.808 | 2.648 |
| 512 | 684 | 1.892 | 2.690 |
| 4,096 | 684 | 2.237 | 3.068 |
| 16,384 | 682 | 4.092 | 5.186 |
| 65,536 | 678 | 9.444 | 11.918 |

The same installed profile build then completed all 4,096 encrypted echoes with
USB debugging disabled in 16.498 seconds. The host observed `18d1:2d00`, one
accessory interface and no ADB interface; it received the final TLS close
notification. Payload totals remained 58,803,628 bytes each direction. Median
RTTs were 1.929, 1.870, 1.991, 2.354, 4.122 and 9.409 ms for the six payload
sizes above. The tablet's saved result and progress journal independently confirm the host
receipt and actual native bridge retirement; both trials used source ID
`93eb9703c1b39cf7dbb380613e35d70e4dfd04ed84565b94e190fc94cb758918:profile`. This proves encrypted accessory-only synthetic transport,
not display, camera, microphone or speaker operation.

These are stop-and-wait synthetic measurements, not rendering latency or maximum
throughput. An earlier 71-echo partial run had 40–80 ms delays and then timed out
reading a payload. TCP_NODELAY and USB termination were changed together; this
trial does not isolate their individual performance effects or compare against ADB.

## Remaining integration and acceptance

1. Wire implemented identity, approval and authenticated transport contracts into
   persistent trust adapters and production UI. Explicit approval and revocation
   must precede media/control authority; finish native-helper packaging.
2. Qualify native duplex I/O and worker supervision. Preserve quarantine until
   actual native ownership ends, including blocked provider calls, reads and closes.
3. Adapt current local stream endpoints and attachment epochs, leaving media
   contracts and isolated display/camera/audio workers independent of USB APIs.
   Preserve optional ADB fallback and label unvalidated backends unsupported.
4. With debugging disabled, validate display/render acknowledgements, touch,
   webcam, microphone and speaker output separately and concurrently. Respect
   actual Android camera/microphone permission denial and revocation. Require
   audio continuity, camera correctness, independent channel stall/death recovery,
   mid-transfer unplug/reconnect and malformed-input regressions.
5. Compare matched rendering latency, CPU, memory and charging under the selected
   capacities. The successful echo trial establishes none of those media results.

### Concrete broker and media integration

One on-demand per-user connection broker owns each authenticated accessory peer,
independently of display capture. Linux display discovery currently lives in
`host/src/monitor.rs` and `discovery.rs`; `usb/monitor.rs` is the Windows preview.
Camera/audio controllers run separately in GUI/CLI processes, so making the
accessory owner a child of the display session would break camera/audio use
without a display. Native USB ownership belongs to a supervised broker worker;
local registrations use an authenticated OS IPC adapter. Preserve one physical
owner and retain its admission until native retirement is confirmed.

Define independent `PeerId`, `ConnectionLease` (identity, generation, capabilities,
revocation), `ChannelRegistration` (channel, generation, bounded capacity),
`MediaInvitation`, `SessionControl` and `LocalChannelEndpoint` contracts. OS
adapters own sockets/descriptors, never the domain requests. The broker exposes
observe/approve/revoke-peer, register/open/retire-channel and invite/result
operations. Registration authorizes only a caller-owned local endpoint; remote
peers must never select arbitrary host addresses/ports. Each media caller owns
its channel lease; physical link loss revokes all leases for that connection,
while a media worker failure retires only its resource/channel.

The smallest integration retains existing media protocols behind bounded local
bridges, with tokens protecting the local endpoints as well as authenticated TLS
protecting the physical link. This adds socket buffering/copies; measure those
costs before deciding on direct stream/descriptor adapters. Relevant boundaries:

- Host display/control: `session_core.rs::{Spec,Prepared,Runtime}`,
  `stream.rs::StreamServer`, `input.rs::InputServer`, and
  `monitor/preparation.rs`. Replace ADB-specific route preparation through an
  explicit adapter; retain attachment leases, authentication and capture gates.
- Host camera: `camera.rs::{run_native,Session::invited}` and
  `camera/bridge.rs::Bridge`. Extract channel registration/invitation behind a
  shared interface, leaving FFmpeg/output ownership unchanged.
- Host audio: `audio.rs::{run,invited,invite}` uses the same registration contract;
  microphone and speaker channels remain independently owned. Preserve manual
  start, bounded PipeWire workers and their actual retirement checks.
- Android routing: inject endpoint/lease contracts into `VideoTransport`,
  `ControlSession`, `CameraWire`, `AudioCapture` and `AudioPlayback` rather than
  assuming fixed localhost ports. Extend `MediaTask.Display`, `MediaTaskJson`,
  `RemoteDisplay` and `LocalMediaRuns` at their adapter boundaries. Keep separate
  display/camera/microphone/speaker processes; do not move media bytes over Binder.
- Android authority: a private connection-worker service publishes authenticated
  state and validated invitations to `SessionCoordinator`, `CameraBinding` and
  `AudioBinding`. Existing DUMP-protected receivers stay ADB adapters. Pairing is
  not camera/microphone permission, and native accessory metadata is not identity.
- Pairing UI: a separate connection/pairing presentation in Android `MainActivity`
  and the host GUI connection/general panel observes broker state. Approval and
  revocation name the authenticated certificate fingerprint and peer; dedicated
  persistence adapters store trust. Never infer approval from cable attachment.

Connection management cannot be inserted into the existing control WebSocket's
raw byte stream. Define bounded, distinguishable management framing or a typed
control adapter before multiplexing invitations and application control. Terminal
handshake semantics must allow already-in-flight peer envelopes to drain without
reviving retired authority or terminating unrelated channels.

Explicit revocation is required: `SessionCoordinator.applyToken` returns for a
null saved token, so clearing `Prefs.hostToken` does not retire live display or
control. `AudioBinding` consumes invitations by setting their flow to null, so a
null invitation does not stop active audio either. Add generation-scoped revoke
operations that stop the affected session/resource and reject queued callbacks.
`CameraBinding` already retires on null endpoint, but its transport must still
check the connection lease. Preserve `Attachment::Lease`, `ResourceLane`,
`AndroidWorkerRun` and worker-death quarantine rather than admitting replacements
on timeouts alone.

Integrate display/control first, then camera, then microphone/speaker, followed
by independent and concurrent acceptance with debugging disabled. Keep optional
ADB selection; native backend acceptance and full feature parity remain pending.


### Authenticated protocol selection and cross-language TLS

The experimental TLS adapters now support API 27/28 without calling unavailable
public ALPN methods. All peers exchange exactly ten authenticated bytes,
`42 4c 45 4e 54 55 53 42 00 01` (`BLENTUSB`, zero, version one), after mutual
certificate verification and exact approved-peer pin checks. The independent
`testdata/connection-tls/protocol-v1.bin` fixture fixes those bytes. Each side
writes/flushes its preface and then checks the entire peer preface before exposing
application streams. Missing, truncated or mismatched prefixes fail closed.

Android API 29+ continues to offer and require `blent-usb/1` ALPN. API 27/28 uses
available TLS 1.2/1.3 protocols and the authenticated preface; Rust permits absent
ALPN only through the same preface gate and rejects a negotiated different
protocol. Rust's single ten-second deadline covers handshake, peer proof and
preface, including a silent peer after successful TLS. Android restores the
caller's read timeout after setup; absolute native handshake/write/close
supervision still belongs to the isolated connection worker and is not supplied
by a per-read socket timeout.

Permanent normal Rust/JVM tests include API eligibility, mutual authentication,
wrong identities, wrong/missing/truncated/versioned prefaces, bounded mutations,
sequence-free exact preface consumption, caller timeout preservation and both
Rust endpoint roles' complete setup deadlines. API 27 Robolectric runs establish
owned API-branch behavior with the local JVM TLS provider; they do not qualify a
physical Android 8.1 provider or native accessory lifecycle.

The normal tooling test `scripts/tests/test_secure_connection_interop.py` builds
`host/examples/secure_connection_fixture.rs` offline with `accessory-transport`,
then executes the explicit `:app:tlsInteropTest` Gradle task. The task runs the
maintained Android `SecureConnection` against the maintained Rust `accept` path
at Robolectric API 27 and 34, including wrong pinned identity and a fault-injected
wrong authenticated preface. The fixture binds an ephemeral loopback port, uses
only committed public test identities and has an absolute twenty-second lifetime.
The orchestrator bounds command/process lifetimes and preserves failure output.
Missing prerequisites fail this test; it has no conditional skips. Ordinary JVM
unit tests remain Rust-independent: this integration class is compiled with their
friend paths but executed only by its dedicated task and normal tooling driver.
No production APK includes the fixture or integration class.

This establishes real Rust/JSSE adapter interoperability, not physical Android
TLS-provider or USB acceptance. Native API 27/28 checks, persisted trust/UI,
revocation and full channel/media integration remain T846 work.
