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

The scheduler carries DATA only. T849 starts only the Management channel active.
Each resource channel rejects offers, releases and incoming DATA/CREDIT until its
current generation is explicitly activated. Terminal closure/drain remains valid;
replacement resets activation and grants no credit. The synthetic probe explicitly
activates its test channels after authentication; this is not production media
consent. The multiplexer reserves one coalesced credit
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
Persistent trust storage is implemented under T868 below; production approval UI remains pending.

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
caller; parsing a certificate does not grant approval. Persistent trust, explicit
revocation policy and authenticated admission are implemented under T868/T852 below.
Production approval UI and broker/service runtime wiring remain unimplemented. The optional Cargo
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

## Delivery milestones after decomposition

T846 previously combined transport research, security, production routing, all media,
packaging and acceptance. These are separate deliverables in `TODO.md`; splitting
them does not count as finishing them. Current production still uses ADB. The
transport foundation is committed at `ffdef93b`; native evidence proves encrypted
synthetic transfer without debugging, not media integration. T849 adds the explicit
activation prerequisite with its own regressions and commit.

| Item | Deliverable | Main prerequisites |
| --- | --- | --- |
| T849 | Explicit channel activation and replacement gate | Transport foundation |
| T850 | Bounded typed management records and incremental parsing | Existing framing |
| T851 | Resource transactions, leases and revocation policy | T849, T850 |
| T856 | Transport-independent peer identities and leased local routes | Existing attachment/media contracts |
| T867 | Durable host identity (implemented Linux adapter) | Existing stock TLS identity validation |
| T868 | Bounded authenticated peer-trust persistence on both sides | T867 storage patterns, existing pairing contracts |
| T852 | Compose stored identities and trust with authenticated pairing | T867, T868 |
| T871 | Local registration contract and Linux descriptor adapter (implemented) | T856 |
| T873 | Cancellation-safe bounded frame driver (implemented) | T849 |
| T872 | Authenticated host resource runtime and retirement barriers (implemented) | T849–T852, T871, T873 |
| T853 | On-demand host broker and client composition | T852, T871, T872 |
| T854 | Isolated Android connection service and local bridges | T849–T851, T856 |
| T855 | Production approve/deny/remember/revoke presentation | T852–T854 |
| T857 | Display and touch/mouse integration | T853–T856 |
| T858 | Independent camera integration; second user-visible milestone | T857 routing; maintainer priority after working display |
| T860 | Independent speaker integration | T857 routing; establishes shared audio adapter |
| T859 | Independent microphone integration | T860 shared audio adapter, T858 consent/uplink patterns |
| T862 | Integrated failure, isolation and reconnect qualification | T855–T860 |
| T846 | Local-build debugging-off media acceptance and matched measurements | Above integration and qualification; T861 not required |
| T861 | Deferred by maintainer: native helper/broker and APK packaging | Resume only on explicit request |
| T863 | API 27/28 native accessory/TLS qualification | Missing suitable native device/runner |

The next user-visible milestone is production display plus touch without ADB
(T857). The maintainer explicitly selected webcam second (T858), followed by
speakers (T860) and microphone (T859). Speaker delivery establishes the shared
audio adapter; microphone reuses it and the camera consent/uplink pattern.
Run lifecycle/failure regressions with each feature and finish concurrent-resource
qualification under T862 before final measured acceptance. Packaging T861 was deferred by the maintainer on October 10 and does
not block local-build implementation or acceptance; no release packaging pass is
claimed. API 27/28 JVM tests do not satisfy T863; its missing physical provider
validation is explicit. No completion percentage or release date is inferred from
item counts: broker/lifecycle/media wiring is substantial remaining work.

### Implemented management activation policy (T851)

The T850 management record schema and T851 portable resource policy are implemented;
production broker/worker adapters remain pending. The schema carries
bounded capability, Offer, Prepared, Commit, Result and Revoke records; remote
records never contain arbitrary host addresses, ports or paths. Each resource
has at most one bounded pending transaction. A display resource jointly owns its
control and display channels; camera, microphone and speaker own separate leases.
A local lease binds authenticated peer, local connection epoch, resource,
channel generation and registration owner. Wire IDs alone grant no authority.

Prepared enables no traffic. On the sender, the entire Commit record must finish
writing and its borrow must retire before the serialized owner installs the agreed
generation and activates channels. On the receiver, processing the complete Commit
installs that generation before another envelope is dispatched. Revoked/expired
transactions install it closed for draining, without granting credit or authority. Peer
events arriving before sender completion handling stay bounded and queued; an
independent completion callback must not race activation. New-generation CREDIT
cannot overtake this barrier. Revocation invalidates authority before worker
cleanup; replacement still requires actual ownership retirement.

`common/src/connection/resources{.rs,/}` and Android `ConnectionResources` retain
exactly four slots. Each lease binds the authenticated connection ticket (peer and
local epoch), resource, host-allocated request/generation, and local registration
owner. Adapters construct the policy only after verified pairing, recheck the exact
lease after asynchronous work, and disconnect it before dispatching callbacks after
link/trust loss. Wire identifiers alone cannot supply local registration authority.

The host offers a resource. The tablet may report PermissionRequired once without
extending the original 1..600,000 ms negotiation deadline. After local permission
and preparation, it commits Prepared to bounded FIFO management output. The host
starts Commit only after receiving Prepared. A separate whole-record completion
callback establishes the installation barrier. The tablet's Result(Admitted) grants
a current lease; this is not evidence that rendering or media playback works.
Pre-Prepared rejection uses Unavailable, PermissionDenied, Busy or Failed. Failure
after Prepared uses Revoke. Duplicate/out-of-order operations and stale callbacks
are rejected before mutating authority; valid late peer responses drain without
restoring a revoked lease.

The host coordinates terminal settlement. Either side can revoke immediately, but
the host must finish any started Commit before sending its Revoke. A tablet that
already cancelled still accepts the preceding expected Commit, installs its agreed
generation closed, and drains in-flight credit. It never starts media. Once the
tablet receives host Revoke, no later Commit is valid. Result(Revoked) is reserved
exclusively for the tablet's final acknowledgement; using it as an unsolicited
rejection could confuse an earlier queued result with settlement proof.

The tablet may queue that acknowledgement only after its native owner and, when
installed, all resource channel consumers/writers and both terminals actually
retire. It retains its tombstone until the entire acknowledgement finishes writing.
The host processes that response behind its own whole-Revoke write barrier and
also waits for local actual retirement before freeing its slot. Display includes
both control and display channels. A kill request, deadline or UI dismissal does
not establish retirement. A stuck owner retains only its resource's slot; other
resources continue. Link loss waives peer settlement, but still requires actual
local native/borrow retirement. The disconnected policy cannot admit another offer.

Never-installed attempts retain the mux's untouched inactive generation; only the
request advances. Installed and fully retired attempts advance the generation.
Counters exhaust without wrapping. Adapter output admission must be reserved before
outgoing transitions, earlier records cannot be reordered through cancellation,
and failed/partial writes disconnect rather than roll back possibly transmitted
state. These are explicit production adapter obligations under T853/T854.
[Retained T851 validation](artifacts/2026-10-10-t851/README.md) includes actual
mux composition in both languages; it does not establish production media support.

### Implemented management record version 1 (T850)

`common/src/connection/management{.rs,/}` and Android `ConnectionManagement`,
`ManagementProfiles` and `ManagementDecoder` define the independent contract.
`testdata/connection-management-v1.json` supplies shared valid/invalid vectors.
One decoder owns a fixed 284-byte scratch buffer; each feed consumes at most one
complete record and returns its exact consumed byte count. Callers retain any
coalesced remainder. Header validation precedes body consumption. Malformed input
poisons the decoder; truncated EOF and reuse after EOF fail without resynchronizing.
Neither mux DATA boundaries nor queued-buffer boundaries delimit records.

The fixed 28-byte big-endian header is `BLMG`, version 1, one-byte operation,
one-byte resource, a zero reserved byte, 64-bit request, 64-bit generation and
32-bit body length. Resource IDs are display session=1 (control+display), camera=2,
microphone=3 and speakers=4. Except capabilities, request/generation are positive
signed-64-bit-range values. Unknown operations, resources and reserved bits fail.

| Operation | Code | Exact body |
| --- | --- | --- |
| Capabilities | 0 | One byte, four resource capability bits; resource/request/generation all zero |
| Offer | 1 | 32 credential bytes plus resource-specific profile |
| Prepared | 2 | Empty |
| Commit | 3 | Empty |
| Result | 4 | One outcome byte, 0 through 6 |
| Revoke | 5 | One reason byte, 0 through 4 |

Display has no extra profile bytes. Camera has 13: width/height as u16, fps as u8,
bitrate/minimum bitrate/freshness milliseconds as u16, lens as u8, then flags
(background bit 0, adaptive bitrate bit 1). Dimensions must be even within
160–1920 by 120–1080, fps 5–30, bitrates 256–20000 kbit/s, freshness 50–2000 ms
and lens 0/1. Host-side rotation and mirroring stay local. Audio has four profile
bytes: processing 1/2, u16 buffer milliseconds (20–200, steps of 10), boolean
background 0/1. Direction follows resource; profile/resource mismatches fail.

Outcomes in order are admitted, unavailable, permission required, permission
denied, busy, revoked and failed. Reasons are requested, connection lost, worker
failed, timed out and permission revoked. Admitted does not mean streaming.
Credential storage is privately owned and omitted from diagnostic formatting;
no remote endpoint address, port, path or executable is representable.

[Retained T850 validation](artifacts/2026-10-10-t850/README.md) proves codecs and
bounded parsing only. Brokers still must supply authenticated identity, enforce
leases/permissions and perform actual resource preparation and retirement.

## Remaining integration and acceptance

1. Wire implemented identity, approval and authenticated transport contracts into
   persistent trust adapters and production UI. Explicit approval and revocation
   must precede media/control authority. Native-helper packaging is deferred under
   T861 and does not gate local-build integration/testing.
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

### Portable local routes and peer metadata (T856)

`common::connection::routes` separates observed attachment continuity from an
authenticated certificate identity. ADB's `device:` profile-prefix interpretation
and transport classification live in `host/src/attachment/adb.rs`; the attachment
owner consumes explicit identity/profile metadata and a portable USB/network link.
Existing ADB profile keys remain unchanged. Native profiles use a separate
`certificate:<fingerprint>` key; no serial-to-certificate identity alias is guessed.
Route handoffs still invalidate accepted leases before forwarding/metadata changes.

The local `RouteClaims` contract admits 1..1024 registrations at the configured
capacity, each with one to five distinct local ports. A bounded ordered map indexes
port ownership and registrations. Admission rejects overlapping ports, duplicate
owners and invalid port/owner bounds without consuming a revision. Revocation
immediately invalidates a claim; reuse requires actual listener/bridge retirement.
Revisions never wrap. This registry does not bind sockets or authenticate IPC;
the T853 broker must own one registry per local network namespace and still bind
each exact endpoint exclusively through its OS adapter.

Android `DisplayRoutes` carries an immutable local revision, credential and distinct
video/control ports through the display interface and private worker IPC. The ADB
adapter alone selects the existing 8890/8891 defaults. Transports construct loopback
socket addresses from the selected local ports and snapshot credentials with the
same connection attempt. Explicit route replacement invalidates old generations;
null routes disable reconnection instead of silently selecting ADB. The revision is
a local registration value, not TLS proof: T854/T857 must map it to a current T851
lease before publishing native routes. Private IPC validates positive integer
revisions/ports and rejects fractional, string or overflowing numbers.

The earlier T864 regression fixes a separate credential race: a pending old control
socket can no longer authenticate with a token published for its replacement.
Native service listeners, production route selection and pairing presentation remain
T853–T857 work. Interface injection does not enable ADB-free production display.

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

Integrate working display/control first, webcam second, then speakers and microphone,
following the maintainer’s October 10 priority. Each item
includes independent lifecycle and debugging-off acceptance; T862 completes
concurrent-resource failure acceptance and T846 matched final measurements. Keep optional
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

## T867 durable host identity

`secure_connection::identity_store` implements bounded certificate/key persistence
behind an independent storage interface. Its Linux adapter holds an exclusive stable
lock in a descriptor-pinned private directory. A durable pending record precedes
generation; complete state is synced and atomically published without replacement.
Malformed, unsafe or interrupted state fails closed and remains available for explicit
repair. Existing keys never silently rotate. [Validation](artifacts/2026-10-10-t867/README.md)
covers restart, concurrent ownership, bounded invalid input and crash/failure boundaries.

This is storage readiness only: T868 supplies peer trust, T852 composes TLS/pairing,
and T853 owns the production broker lifecycle and directory selection. Windows has
no implementation of this storage adapter yet. Working display/touch remains first
production milestone, followed immediately by webcam, then speakers and microphone.

## T868 authenticated peer-trust persistence

Rust `connection::trust` and Android `ConnectionTrust` now implement bounded saved
approval behind independent storage contracts. Shared canonical records bind exact
peer pins to the local certificate and a nonwrapping revision. Capacity defaults to
64, configurable from 1 to 1024; reducing capacity never silently evicts saved peers.
A remembered approval is written only after the current pairing lease proves TLS
authentication. Revocation invalidates live pairing first; a failed write poisons
reconnect admission. A multi-link broker must revoke all matching pairings in its
single serialized command owner. Saved approval alone never grants media authority.

Linux and Android private-file adapters retain exclusive stable locks and reject
interrupted pending/temp state. A durable intent marker precedes complete atomic
publication; synchronized publication precedes marker removal. Failure before intent
durability cannot promise persistence, and no failure reports a successful revoke.
Both adapters preserve existing evidence for explicit repair. [Validation](artifacts/2026-10-10-t868/README.md)
includes shared malformed vectors, per-function counters and an actual API36 tablet
filesystem probe run without installing/restarting the app. T852 still composes these
stores with stock TLS; production presentation and resources remain separate work.

## T852 persisted pairing composition

Both trust policies now expose admission operations that select persisted approval
and complete current authenticated pairing. A saved pin still requires fresh stock
TLS proof. If proof or remembered persistence fails, the matching attempt closes;
a stale callback cannot cancel a newer attempt. Native retirement remains required
before replacement. This reuses existing policy/storage boundaries without adding
a separate broker or UI dependency to the contracts.

[Integration evidence](artifacts/2026-10-10-t852/README.md) demonstrates persisted host
identity reuse through real mutual TLS, restart/reconnect, explicit revoke and wrong
certificate rejection. Android's stock adapter interoperates with Rust while saving
and reopening trust on API27/34 JVM fixtures. Production broker/service/UI and media
remain T853–T858; no current ADB-free production-display support is claimed.

## T871 local registration boundary

The portable BLRG version-1 registration protocol admits bounded Register,
Unregister, Registered, Status and Rejected packets. Register carries the peer
pin, resource, credential and existing resource profile; it never accepts claimed
ports, paths or process IDs as endpoint authority. Display supplies two listener
capabilities in video/control order; other resources supply one. Local admission
is not authenticated remote media admission.

The Linux adapter uses nonblocking Unix SOCK_SEQPACKET, authenticates both peers
with SO_PEERCRED, and transfers descriptors using SCM_RIGHTS/CLOEXEC. It derives
ports from exclusive TCP listeners bound to 127.0.0.1, rejects shared ports and
other socket types/protocols, and retains capabilities until explicit retirement.
A wrong or still-active route cannot release its listeners. Capture reuses the
same bounded ancillary transport, preserving its existing malformed-packet tests
and allocation-free send path.

[Validation](artifacts/2026-10-10-t871/README.md) covers bounded malformed inputs,
ancillary cleanup, independent callers, ownership and retirement. T872 owns
authenticated media dispatch; T853 owns private runtime directory selection,
election, socket cleanup and the production broker/client lifecycle. No broker
executable or working ADB-free production display is claimed by this boundary.

## T873 asynchronous frame I/O

The Rust transport driver preserves partial frame offsets and absolute active-I/O
deadlines across cancelled waits, services read/write directions independently,
and reborrows existing outbound queue buffers instead of copying them into a
second payload staging buffer. Receive staging is fixed at 64 KiB; deadline
timers are reused. Completed write events follow flush and writer retirement.
Completed receive bodies remain available until explicit consume, and delivery
is rechecked after partial-body revocation.

[Scoped tests/counters](artifacts/2026-10-10-t873/README.md) pass. T872 must compose
these frame events into whole management-record publication barriers and resource
leases, including inbound events arriving while TLS flush remains pending. The
driver alone does not perform admission, native-owner retirement or media routing.

## T872 host resource runtime

The host Session now composes framed I/O, the management decoder and Resources.
Whole-record Commit/Revoke write completions precede the associated transitions.
Media flow validation for a committing generation waits behind that barrier,
while management credit continues; matching early results have one deferred slot
per resource. Invalid/stale protocol events disconnect the link.

Bounded per-channel receive rings and generation-scoped consumer borrow IDs keep
credit charged until actual output completion. Five coalesced readiness slots
avoid notification growth and transport is polled before notifications, preserving
revocation progress during tiny reads. Full resource leases validate callbacks;
local cancellation never substitutes for native retirement. Connection retains
native completion independently and rechecks the current verified pairing.

Production wire session ID is 1 inside each fresh mutually authenticated TLS
stream. Local Pairing epochs are independent on each device and never become
wire session IDs. TLS resumption remains disabled. T853/T854 must agree the
symmetric receive window over authenticated TLS before initial CREDIT and drive
idle `tick` deadlines. Capacity configuration alone is not that agreement.

[Validation and retained regressions](artifacts/2026-10-10-t872/README.md) include
stock TLS duplex admission/control and actual isolated helper exit. The broker
executable/client/bridge composition and Android service remain unimplemented;
production display/touch remains the next native acceptance milestone.

### T874 private host IPC owner

The Linux broker library now elects one owner using a stable private lock inode
inside an existing owned mode-0700 runtime directory. The directory descriptor
pins socket creation and cleanup across rename. Only an owned stale socket is
removed after election; ordinary shutdown removes only the same socket inode.
The lock file is never unlinked. This does not sandbox hostile same-user code.
Async packet adapters permit one reader and one writer concurrently, retain
borrowed listener descriptors until atomic send, and close received descriptors
on malformed packets. Shutdown revokes I/O; actual descriptor ownership ends
only when the wrapper and outstanding borrows drop. Runtime-directory selection,
on-demand process startup and registration orchestration remain T853.

### T875 authenticated runtime-window agreement

Before either endpoint constructs its production multiplexed Session or sends
CREDIT, the already mutually authenticated TLS stream exchanges exactly 12 bytes
per side: `BLCP`, version byte 1, role byte (host 1, tablet 0), two zero reserved
bytes and a big-endian u32 capacity. Host sends its configured capacity first.
Tablet chooses the smaller of that proposal and its configured capacity and
replies with the selection. Both use that selection in both directions. Valid
capacities are 4096..1048576 bytes. Host rejects a selection exceeding its proposal.
No fallback or implicit equality is permitted. The existing production wire
namespace remains 1 inside each fresh TLS stream.

The host adapter uses one absolute I/O deadline (configured 1 ns..60 seconds,
default five seconds) covering proposal write, flush and complete reply. Error
or cancellation invalidates the setup stream; its owner must drop it, retain
native retirement evidence and recheck the pairing lease before installation.
Remaining queue/quantum/deadline settings stay local. Parsing this record grants
no approval or media authority. T853 must invoke this adapter; T854 implements
the matching Android exchange. Tests use isolated byte streams; production media
and debugging-off acceptance remain T855/T857 and subsequent media items.

### T877 local pairing and discovery control

Authenticated local BLRG v1 IPC adds operation 5 (typed Control command) and
operation 6 (typed Report notice), both carrying zero descriptors. Existing
resource-registration operations and capability validation remain unchanged.
Each control body is at most 128 bytes within the existing 4096-byte packet cap;
request/counter fields are 1..i64::MAX and every extent is exact.

Commands are Inspect, Connect(revision,target), Decide(connection,ticket,allow,
remember), Disconnect(connection,optional ticket), and Revoke(peer). Denial
cannot remember trust. Notice snapshots contain Begin(revision,computer peer),
Device(revision,opaque target,VID,PID,observed accessory), Link(revision,connection,
optional ticket,state,saved trust), Saved(revision,peer), and End(revision).
Pending/approved/authenticated pairing states carry an exact certificate and
local epoch. Discovery has no ticket; closing/quarantined states may precede or
follow certificate discovery. Observed USB IDs never grant authority.

The T853 owner must issue nonrepeating IDs during its lifetime, validate the
current snapshot/selection and exact approval ticket, bound client snapshot
publication and reject stale callbacks. A new IPC connection after broker restart
invalidates cached IDs; clients must not replay commands against a fresh owner.
Snapshot Begin/End revisions must match before presentation uses the result.
These codecs do not themselves approve, connect or revoke anything. T855 consumes
this contract for production presentation after T853 orchestration exists.

### T876 read-only Linux discovery

`blent-usb-io --list LIMIT` provides bounded inventory without ADB, device opens,
string queries, interface claims or accessory mode switches. The stock libusb
adapter emits BLUI v1: 16-byte header (eight-byte magic, big-endian count u16, six
zero reserved bytes), then 16-byte records (bus, address, port depth, device class,
VID/PID as big-endian u16, seven zero-padded port numbers, final zero reserved byte).
The maximum is 512 candidates; capacity overflow fails instead of truncating.
Root controllers and hubs are excluded. Other devices have unknown support unless
already reporting the stock accessory VID/PID; this observation grants no trust.
The Rust Scan adapter bounds bytes/time and retains independent native Completion.
T853 still maps this native data to opaque IDs and revalidates explicit selections.

### T878 local endpoint pumping

A portable Endpoint boundary carries exact resource lease/channel identity,
current authority, outbound allocation capacity and consumer borrow/completion
operations. Session implements it. The host Pump owns one local byte-stream
handle and bounded state for both directions; it never spawns per-block tasks.
Configured producer blocks are capped to the endpoint's retained-allocation
capacity. One pending consumer borrow holds credit until complete write/flush.
Independent active-I/O deadlines do not charge idle socket reads. Errors poison
only this pump. The owner must still cancel the whole affected resource, retire
all its channel handles and observe any independent native worker Completion.

The T853 reactor must poll pumps after local socket readiness AND Session progress
(including freed queue capacity), and continue polling Session while consumers
stall. It must schedule idle resource ticks. A constructor, buffer drop, cancelled
wait or kill request cannot establish native resource retirement. These library
boundaries are tested but do not themselves enable production media without ADB.
