# T846 connection contract and integration boundary

The current Android connection composition is described under T854, pairing
presentation under T892/T893, and local display/Activity routing under T905/T906/
T909/T907 below. Host presentation and controls are
implemented under T900/T901/T903/T894; joined physical pairing acceptance remains T855. Validation counts
in individual milestone sections describe their historical scoped runs; they are
not whole-project coverage or physical-device support claims.

## Implemented domain layer

`common/src/connection/` and Android `com.blent.connection` implement the same
bounded envelope, duplex credit state, fair multiplexer, public identity discovery and pairing authority state machine. These are
portable domain components; native production composition is described below.
Their constructors and wire values alone confer no pairing or media authority.

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
The Linux broker runtime is implemented under T853 below. Production approval UI and Android service wiring remain pending. The optional Cargo
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
| T853 | On-demand Linux host broker and client composition (implemented) | T852, T871–T879 |
| T881 | Android bounded framing and BLCP negotiation (implemented) | T849, T875 |
| T882 | Android tablet resource-session and leased-buffer boundary (implemented) | T881, T849–T851 |
| T883 | Bounded Android native session runner (implemented) | T881, T882 |
| T884 | Private Android connection process and IPC (implemented) | T883 |
| T886 | Authenticated Android setup owner (implemented) | T884, T851, T856 |
| T887 | Android accessory selection/permission/opening (implemented) | T884 |
| T888 | Authenticated Android runtime supervision (implemented) | T886, T883 |
| T889 | Supervised Android USB admission (implemented) | T887 |
| T890 | Configured connection shutdown budgets (implemented) | T884 |
| T854 | Production Android accessory/setup composition (implemented) | T888–T890, T884 |
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
item counts: Android service, presentation, lifecycle and media integration remain substantial work.

### Implemented management activation policy (T851)

The T850 management record schema and T851 portable resource policy are implemented;
the host broker adapter is implemented under T853; Android worker integration remains pending. The schema carries
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
and reopening trust on API27/34 JVM fixtures. The host broker is implemented under T853. Android service/UI and media
remain T854–T858; no current ADB-free production-display support is claimed.

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
executable/client/bridge composition is implemented under T853; Android service remains pending;
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
only when the wrapper and outstanding borrows drop. T853 implements explicit private runtime/state paths,
on-demand process startup and registration orchestration.

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
no approval or media authority. T853 invokes this adapter; T854 supplies
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
this contract for production presentation using the T853 orchestration boundary.

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
T853 maps this native data to opaque IDs and uses the T879 selected-device metadata gate before any accessory switch.

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

## T853 on-demand Linux broker composition

`blent-usb-broker` now composes the discovery, private local IPC, persisted
identity/trust, stock mutual TLS, receive-window negotiation and bounded resource
bridges. Build locally with `cargo build --offline --locked -p blent --features
accessory-transport --bin blent-usb-broker`. Launch requires absolute `--runtime`,
`--state` and `--helper` paths. Runtime/state directories must be private and owned
by the user; the broker creates missing directories with mode 0700. The helper
must be the locally built `blent-usb-io` executable. No helper is downloaded,
installed or selected from peer input. Packaging remains deferred under T861.

The client `linux::client::Connector` starts the executable on demand and shares
one elected owner across callers. Caller loss revokes only that caller's routes;
it does not terminate the shared USB connection or other callers. Discovery runs
without switching device mode. A current explicit Connect selects an opaque
inventory target; the native helper rechecks its observed address/VID/PID before
switching. Only current approval plus fresh TLS proof grants media authority.

The portable Controller owns tickets and lifecycle policy. A separate bounded
native worker exclusively owns trust storage, so blocked persistence cannot stall
existing media. Revocation immediately closes matching live links, and its IPC
completion waits for durable deletion. Saved notices continue reflecting durable
storage until that operation succeeds. Each link has its own asynchronous owner
and native helper admission. Cancellation does not release native capacity; task
completion and actual helper completion are independent retirement conditions.

Only authenticated local registrations can provide loopback listener descriptors.
Connections to those listeners begin after the corresponding resource becomes
current. Display uses ordered video/control listeners; camera, microphone and
speakers each use one. Bounded pumps continue independently when another local
endpoint stalls. Sockets close before route capability retirement. The broker's
control IPC carries metadata and descriptors, never media payloads.

Capacity options and defaults are: `--clients 16`, `--links 4`, `--devices 64`,
`--registrations 64`, `--trust-peers 64`, `--block-bytes 65536`,
`--channel-bytes 262144`, `--queued-messages 64`, and `--quantum 65536`.
Higher limits permit more concurrent work and retained memory. Receive windows
are negotiated to the smaller peer limit; producer blocks respect that window.
Deadlines are `--io-timeout-ms 5000`, `--setup-ms 60000`, and
`--approval-ms 120000`; approval permits up to 600000 ms and reaches native READY
coordination, while the controller retains the original absolute deadline.
Inventory refresh is two seconds with one five-second native scan at a time.
Failed scans explicitly report unavailable inventory. SIGINT/SIGTERM stop new
work and allow five seconds for actual retirement before process exit.

Permanent isolated tests exercise the executable, owner election, client loss,
identity restart, real mutual TLS, durable trust/revoke, descriptor-only routing,
duplex control bytes and stuck native shutdown. These tests do not establish
debugging-off production media acceptance: Android service T854, pairing UI T855
and display/touch T857 remain required. Linux is the implemented broker backend;
Windows runtime support is unavailable, and macOS remains declined.

## T881 Android transport ownership

Android now implements the same BLCP v1 receive-window contract as the host.
`ConnectionWindowNegotiation` consumes only the 12-byte proposal, selects the
smaller valid capacity, and publishes it only after the whole reply and its
flush complete. Exact opaque receipts reject stale callbacks. A single absolute
deadline spans proposal, reply and flush; byte progress does not extend it.

`ConnectionFrameReader` and `ConnectionFrameWriter` are separate portable state
owners. Native adapters receive one exact array-range loan per operation and
return it only after that operation actually finishes. Header/body offsets
survive short reads/writes and zero progress. The reader has a fixed 64 KiB body
buffer; the writer borrows the caller-owned payload without another full-frame
copy. A completed receive frame has one consumer claim, remains retained until
release and cannot be retrieved after cancellation. Cancellation prevents reuse
but keeps outstanding native loans visible until their exact completion.

Active frame deadlines span the complete header/body and, for writes, flush.
An idle reader has no active-frame deadline. A supervisor must tick deadlines
while native calls block and retain process/worker ownership through actual
retirement; these pure state machines do not interrupt syscalls or establish
native worker exit. Authenticated identity, media consent, resource leases and
receive-credit release remain the session owner's responsibilities.

[Evidence](artifacts/2026-10-10-t881/README.md) covers permanent cancelled/duplicate
loan and frame-claim regressions, fragmented/coalesced frames, exact extents,
bounded mutation, deadlines and independent directions. Actual maintained Rust
and Android TLS adapters exchange BLCP plus management CREDIT/DATA using the new
framing on API27/34 JVM fixtures. Production resource dispatch is T882; native
service ownership is T854. JVM interoperability does not satisfy physical-device
acceptance or the API27/28 native-provider gap T863.

## T882 Android resource-session boundary

`ConnectionTicket`, `ConnectionApproval` and `ConnectionResourceLease` are now
independent contract values. Pairing, persisted trust and resource policy use
those values without exposing their implementation classes in session/endpoint
signatures. Constructing a value still supplies no authentication proof.

The portable `ConnectionSession` contract is implemented by `TabletSession`.
Its single serialized owner composes management decoding, resource policy and
multiplexing after authenticated pairing and the completed BLCP window agreement.
Only the trusted native adapter may supply that authenticated ticket. Offer
publication reports a validated profile and owned credential; permission wait,
preparation and host Commit remain distinct transitions. Media authority begins
only after Commit admission. Declined or timed-out resources close independently;
a late Commit installs a draining generation without granting DATA credit.

Display owns video and control together. Camera, microphone and speakers have
independent lanes. Five lazily allocated receive rings each use the agreed window;
a contiguous consumer read borrows existing storage rather than copying it.
Credit returns only after the exact consumer receipt completes. Outbound array
admission transfers ownership to the session; the caller must not mutate or reuse
an accepted array. Queue byte/message limits provide backpressure independently
of inbound credit. Self-aliased ring pushes are rejected before any mutation.

Management output has 16 bounded publication slots, with one complete record
admitted to the mux at a time. A Revoke ACK takes effect only after every fragment
and the actual writer completion. One replacement Offer may wait behind that
barrier: the host can receive the ACK before the tablet's flush callback runs.
Channel generation replacement also requires peer terminal, writer and local
consumer retirement. Disconnect revokes authority immediately but does not free
native loans; transport exit and each local endpoint's actual retirement remain
separate, explicit evidence. Failed exact writer completions poison the session;
foreign or duplicate completion receipts fail before changing ownership.

Normal state-event admission stops at 28 events. Eight reserved cleanup positions
permit Closing and Retired for every resource even if the event consumer stalls;
the hard bound is 36. Readable notifications instead coalesce in five fixed slots
and rotate across channels. Resource scans iterate the fixed four-slot array,
without allocating a temporary collection per received/written frame.

Capacity defaults are a 256 KiB window, 64 queued messages per lane, 64 KiB
scheduling quantum and 120-second resource setup timeout. Supported ranges are
4 KiB–1 MiB, 1–1024 messages, 1–65536 bytes and 1–600000 milliseconds respectively.
Larger windows retain more bytes when consumers stall; smaller quanta increase
scheduling/framing work. These are bounded configuration choices, not throughput
claims. No new performance timings or physical-device acceptance are implied.

T854 supplies native accessory/service ownership and process-death quarantine.
Locally allocated listeners remain T857; Binder presentation remains T855. Production media adoption and
actual debugging-off acceptance remain T855/T857–T860. The portable session alone
does not enable the production application's ADB-free connection.

## T883 bounded Android native session runner

`ConnectionEndpoints` separates resource commands and buffer loans from the
framed protocol interface. `ConnectionRun` exposes that endpoint interface plus
transport progress, failure, stop and retirement state. `ConnectionIo` defines
exact native requests/completions independently of stock streams or Android
services. The protocol owner and its native adapter remain separate modules.

`ConnectionSessionRunner` serializes framing and session progress. One read and
one write/flush may be outstanding; each completion names its exact request.
Partial/zero progress preserves the active frame's absolute deadline. Invalid
counts, malformed frames, foreign callbacks and native failures stop protocol
admission. Completed writes reach session publication only after actual flush
completion. The owner handles at most two native completions per advance.

`StreamConnectionIo` owns three reusable daemon workers: read, write/flush and
close. Each direction retains one bounded slot through completion consumption;
there is no executor queue or new thread per chunk. Native close never runs on
the protocol owner. A blocked direction cannot stop the sibling or supervisor
clock. The optional concurrent `ready` callback is an enqueue-only wake hint;
it must not invoke the serialized reactor directly and may run during startup.
A bounded conflated wake plus an independent timer avoids adding timer latency
to every header/body/flush and still observes blocked operations.

Cancellation immediately revokes session authority and native admission. Exact
callbacks after cancellation retire ownership without delivering media. A final
callback racing actual native exit cannot restart protocol processing or falsely
fail a clean stop. Interrupted idle workers fail closed instead of silently losing
a direction. Partial worker-start failures remain observable and owned; inability
to start the closer leaves an explicitly unretired owner.

The default active-frame timeout and cleanup deadline are each five seconds,
configurable from 1–60000 milliseconds. Cleanup timeout sets a process-termination
request; it never marks native I/O retired. Retirement requires actual read,
write and close worker exits. A close failure also requests process termination,
even after the calls have exited. The containing service must retain physical
admission until actual process death; this runner cannot establish OS handle
release, local endpoint exit, or Binder death on its own.

Private process/IPC supervision is implemented under T884; T854 now composes
USB discovery/permission, identity/trust and TLS setup with that service.
Locally allocated endpoint pumps and debugging-off display acceptance remain
T857. Physical native-stream acceptance is not established by portable or JVM
tests.


## T884 private Android connection process and control boundary

`ConnectionWorker` defines lifecycle and bounded command admission independently
of Android Binder, USB, TLS and storage. Typed configuration, pairing decisions,
trust updates and connection events use a flat versioned codec. Encoded metadata
is limited to 4 KiB; the complete Parcel is limited to 8 KiB. Strict generation,
boolean, string, opcode and trailing-data checks reject malformed envelopes.
Neither contract nor codec transports media buffers.

`ConnectionWorkerService` runs in the nonexported same-UID `:connection` process.
It accepts one generation and one START for its lifetime. Same-UID one-way Binder
calls only validate and admit bounded control work; worker construction, native
execution, event delivery and cleanup run outside Binder callbacks. Commands and
events each have 32 slots. Callback loss, overflow or rejected delivery requests
process exit. A failed callback death-link exits before native factory admission.
Main-process death, explicit EXIT, unbinding and service destruction also request
exit; no new native owner can replace a stalled one within the same service.

`AndroidConnectionWorkerRun` owns binding and outgoing calls independently of
its caller's coroutine. START is treated as potentially effective before its
transaction returns. STOP and EXIT have separate asynchronous admissions so a
blocked STOP cannot prevent EXIT. Ended and framework binding invalidation are
execution observations, never substitutes for actual Binder death. Failed
retirement retains the binding and outstanding calls; replacement remains
quarantined until process death and actual local call completion. Generations
fail on exhaustion instead of wrapping. A late Binder still receives STOP/EXIT
while ownership is retained. Retirement without a Binder atomically closes
callback admission before unbinding. Failed death-link registration revokes
START admission before releasing the waiting sender.

These are process/control adapters. T854 now supplies the production factory for
USB selection, permission, identity/trust, approval and TLS setup. T855 supplies presentation, and T857 supplies local media
endpoints and production display/touch acceptance. Robolectric lifecycle fixtures
are permanent automated regressions, not proof of physical USB or kernel Binder
termination behavior. No ADB-free production-media support is claimed here.

Validation: [T884 retained evidence](artifacts/2026-10-10-t884/README.md):
265 connection tests, 85 per-function coverage checks, 116 executable JVM method
counters above 80%, and 134 complexity checks at or below 7.


## T886 authenticated Android setup owner

`ConnectionSetup` is a prompt serialized control boundary; `ConnectionSetupBackend`
and `ConnectionSetupLifetime` define blocking native operations independently of
Android, sockets, USB discovery or storage implementation. `ConnectionSetupOwner`
uses one authority worker for exact pairing/trust mutations and a separate native
closer. A bounded control gate immediately invalidates admission on stop or matching
revocation even while TLS or durable trust publication is blocked. The containing
reactor must supply concurrent wake hints and an independent `advance()` timer.

Setup exchanges bounded public certificates, coordinates current approval with
READY, verifies stock mutual TLS, durably records explicit remember consent and
negotiates the authenticated BLCP receive window. It never treats HELLO, a USB
metadata match or permission approval as peer identity proof. A queued revoke
arriving before the peer is discovered prevents that peer's later admission.

`ConnectionSetupReady` lends its framed I/O handle, approval and selected capacity;
the setup owner retains ownership. Consumers must check `current(approval)` before
dispatch and disconnect immediately after revocation. `TabletSession` has no
injected approval callback; that integration gate belongs to T854. Successful
remember writes emit a durable Trust(true) fact even if cancellation prevents
admission; a subsequent revoke emits Trust(false) only after its deletion succeeds.
Cancellation alone does not undo previously granted remember consent.

`SocketConnectionSetup` composes existing HELLO/READY/TLS/window implementations.
Absolute read deadlines cover fragmented discovery/readiness/window records; the
independent supervisor also bounds blocked native writes, TLS and close. READY
uses the remaining consent deadline; TLS is bounded by both that deadline and
its native-operation budget. Late native completions cannot reset expired phase
deadlines before the next reactor tick. Even unrelated remembered-peer deletion
is bounded after setup is ready. Retirement grace begins at actual stop time. Strict
certificate parsing precedes approval presentation. Late TLS wrappers remain
owned and close when cancellation already began. The single close admission is
shared with framed I/O; failed socket/bridge close cannot establish retirement.
Actual native setup and I/O worker exits remain part of aggregate owner retirement.

Defaults are 10 seconds for each native setup phase, 120 seconds for approval,
2 seconds for retirement and 64 remembered peers. Supported deadline ranges are
1–60000 ms for native/retirement phases and 1–600000 ms for approval; trust capacity
is 1–1024 peers. Larger capacities retain more trust state; longer deadlines delay
failure recovery. These are configuration contracts, not measured performance gains.
T854 exposes them through production worker configuration, using its single shared
5-second retirement default for setup and the session runner. The selected channel
window remains bounded by both peers' configured limits.

Accessory selection/permission/opening is implemented under T887, supervised by
T889 and composed by T854. Pairing UI and physical debugging-off acceptance remain T855,
and actual display/touch routing and acceptance remain T857. Loopback tests do not
establish native AndroidKeyStore, physical USB or real Binder-kill acceptance.

Validation: [T886 retained evidence](artifacts/2026-10-10-t886/README.md):
312 ordinary connection tests and 12 Rust/Android TLS interoperability tests pass;
61 individual source-function checks and 80 executable JVM method counters pass
(minimum 88.89%); 208 reviewed functions have cyclomatic complexity at most 9.

## T887 Android accessory selection and native opening

`AccessoryDescription`, `AccessorySelection`, `AccessoryResolver` and
`AccessorySelectionOwner` keep selection policy independent of Android and native
I/O. Length-prefixed SHA-256 snapshots preserve null versus empty metadata and
reject malformed Unicode and fields larger than 512 UTF-8 bytes. A positive,
nonwrapping epoch plus the snapshot digest forms the canonical worker device
token. Replacement invalidates the previous selection before attempting a new
scan. Stale callbacks cannot invalidate a newer selection. Snapshots default to
32 accessories, with a configurable range of 1–1024; bounds are checked before
traversal. T854 forwards the chosen capacity through composition.

`AndroidAccessoryInventory` re-enumerates an exact, unique supported candidate and
checks actual `UsbManager` permission. Its selection snapshot consistently omits
serial: Android 29+ can require accessory permission to read that field, so using
it before consent prevents permission requests and using it only afterward changes
the selection token. Otherwise identical public metadata is ambiguous even when
serials differ; no unique physical identity is claimed. Native metadata never
grants pairing or media authority. The supported manufacturer/model/version
profile is injected; T854 coordinates the production profile with the host helper.

`AndroidAccessoryPermission` owns one permission request and its native cleanup.
Callbacks use a private action, an immutable package-directed PendingIntent and
an app-signature permission gate on API27/34; API33+ additionally registers a
nonexported receiver. Extras claiming permission are ignored. The receiver only
correlates a current request and admits one background hint. Queue rejection
changes local failure state; it does not perform native cancellation on the
receiver thread. A supervisor must observe `failure()` independently of native
progress. A fresh snapshot and real permission check precede result delivery,
and the main serialized owner must call `confirm` before admitting a worker.

Monotonically increasing request epochs and exact pending receipts reject stale
replacement/completion. Failed cancellation retains the native token for cleanup.
A blocked previous cancellation prevents creation of a replacement token. Late
registration after close is unregistered by its retained native operation.
`retired()` includes actual operation completion, successful unregistration and
release of every pending token; a stop request or reported result is insufficient.

`AndroidAccessoryOpening` owns descriptor acquisition, independent stream wrappers
and the socket bridge. It retains every current native owner before advancing,
checks selection/permission before and after opening, and rejects late/stale
results. Cancellation races close late acquisitions; failed native cleanup cannot
establish retirement. The caller must retain the opening owner even when `open`
throws and must supply its current-selection gate. Native opening and close belong
on supervised workers, with independent timeout/process-retirement enforcement.

These are tested components, not accepted production USB support. T854 composes them
with the private connection service, setup owner and reactor, including permission
failure monitoring, detach invalidation, current-selection/current-approval gates
and actual retirement. T855 supplies pairing presentation and physical acceptance;
T857 supplies production display/touch. Physical detach/reattach remains T855/T862.

Validation: [T887 retained evidence](artifacts/2026-10-10-t887/README.md):
361 connection tests pass without skips; all 47 inventoried production functions
and 78 executable JVM methods pass individual line coverage (100% for these
methods). All 89 reviewed functions have cyclomatic complexity at most 7.

## T888 authenticated runtime supervision

`ConnectionRuntime` and its prompt `ConnectionRunFactory` separate supervision
from Android, sockets and native construction. `ConnectionRuntimeOwner` retains
setup and one returned runner. Its injected monotonic clock bounds dispatch;
invalid/backward/exhausted time closes admission using the last valid cleanup
time. A factory is attempted once. A throwing construction or failed stop keeps
ownership quarantined because partial native effects cannot be ruled out.

A stable `ConnectionEndpoints` facade checks current approval and the retained
connection ticket before new work. `prepared` and `permissionRequired` reject
future/negative timestamps and use fresh runtime time rather than a stale callback
time. Read/offer/current deny admission after revocation; reject also requires an
active session. Exact consumed receipts and native-retirement evidence remain
valid cleanup after stop, with the same connection/lease validation. Cancel can
reach an already-closing retained lease. This preserves `TabletSession`'s existing
post-revocation borrowed-buffer contract without reopening transport authority.

Event delivery is bounded. Stale Offered, Readable and ADMITTED events are filtered;
Closing and Retired events remain available only for the retained connection.
Durable trust facts survive logical cancellation. Clean setup/runner termination
also initiates aggregate cleanup even without a failure. Ended is a request to
finish ownership, never evidence that native calls or the OS process exited.
Actual retirement requires setup and runner completion; external endpoint workers
and private-process death remain separate owners in T854/T857.

T854 implements the production service/native factory, configuration and USB
metadata wiring. Pairing/connection presentation remains T855 and real production media
remains T857–T860. The runtime factory must be prompt and retain partial native
ownership on failure; native I/O, identity loading and trust storage do not belong
inside synchronous runtime construction.

Validation: [T888 retained evidence](artifacts/2026-10-10-t888/README.md):
382 ordinary connection tests and 14 real Rust/Android TLS interoperability tests
pass. All 32 inventoried production functions and 34 executable JVM methods pass
individual line coverage (100% for these methods); 129 reviewed functions have
cyclomatic complexity at most 9.


## T889 permission supervision and T890 shutdown budgets

`AccessoryAdmission` separates the main-process permission supervisor from native
Android APIs. `AccessoryAdmissionOwner` owns one native worker, an independent
closer and a 32-slot callback queue. `AndroidAccessoryAdmissionBackend` owns the
actual detach receiver, inventory and `AndroidAccessoryPermission`. Construction
is inert. The control owner supplies an independent timer; callbacks only enqueue
bounded work or synchronously invalidate selection. USB metadata and permission
never substitute for TLS proof or media consent.

Native request/callback deadlines are separate from the absolute human consent
window. Duplicate/stale permission results cannot publish a second admission.
Stopped late startup cannot begin enumeration. Failed native cleanup, partially
constructed owners and still-running threads retain ownership; cancellation is
not retirement. T854 connects these owners to worker binding, and T855 supplies
presentation and native debugging-off acceptance.

`AndroidConnectionWorkerRun` now derives its default shutdown grace from configured
native retirement plus 1000 ms (maximum supported reactor cadence), then permits
5000 ms for process exit and local cleanup. Existing explicit timing overrides
retain their bounds. A correctly tagged Ended permits prompt EXIT even after STOP;
actual Binder death and finished local calls remain mandatory before retirement.

[T889 evidence](artifacts/2026-10-10-t889/README.md): 45 tests, 48 inventoried
functions and 52 executable JVM methods passing individual coverage (100% line),
103 reviewed functions at cyclomatic complexity <=9. [T890 evidence](artifacts/2026-10-10-t890/README.md):
42 tests, 35 inventoried functions and 40 executable JVM methods passing individual
coverage (100% line), 44 reviewed functions at complexity <=9. These measurements
are scoped framework-model results, not native USB/Binder acceptance.


## T854 production Android composition

`androidUsbConnection` is the main-process entry point. `ConnectionAdmissionRun`
retains T889 permission admission throughout one T890 private-process client.
Native permission must complete before binding; caller-supplied device text cannot
bypass selection. Detach invalidates selection immediately and wakes cancellation.
A separate timer observes admission failure while the child connection is waiting.
Late pairing/status callbacks cannot present stale authority; durable trust facts
remain observable. Both actual permission retirement and actual worker/Binder
retirement are required before replacement. A stuck child does not block the
independent cleanup deadline; unconfirmed ownership stays quarantined.

`ConnectionWorkerService.createWorker` uses `AndroidConnectionFactory`.
Construction captures suppliers only. `AndroidConnectionSetupBackend` opens USB
and the retained socket bridge on the setup authority thread, then loads the
AndroidKeyStore identity and app-private trust storage. Native work, including
late factory results, remains owned through cancellation. Existing identity/trust
files are preserved; an invalid directory or interrupted trust record fails
explicitly instead of silently repairing, rotating or deleting state.

`AndroidConnectionWorker` serializes bounded commands and events with a conflated
wake channel and independent timer. It composes T886 authenticated setup with T888
runtime and T883 session I/O. It advertises zero media capabilities until T857
supplies real endpoint owners. Internal Ended does not imply native retirement;
the containing service reports execution completion after cleanup. Media bytes
never travel through Binder.

Configuration is carried in private same-APK spec schema version 2; command/event
schemas remain version 1. Limits are validated before native work:

| Option | Default | Supported range | Tradeoff |
| --- | --- | --- | --- |
| Native setup/permission operation timeout | 10000 ms | 1–60000 ms | Longer waits tolerate slower native calls but delay recovery |
| Human permission/pairing approval | 120000 ms | 1–600000 ms | Absolute consent window; native calls keep their shorter budget |
| Native retirement | 5000 ms | 1–60000 ms | Shared by setup and session cleanup; longer grace delays quarantine |
| Accessory snapshot capacity | 32 | 1–1024 | Bounds lookup work and retained metadata |
| Remembered peer capacity | 64 | 1–1024 | Bounds trust state; lowering below saved usage fails without deletion |
| Reactor timer cadence | 10 ms | 1–1000 ms | Longer cadence lowers idle wake frequency but delays deadline observation |

Existing per-lane byte/message, scheduling quantum, I/O and resource setup limits
are forwarded unchanged. T890 derives outer process shutdown allowances from the
same native retirement budget. Each main-owner cleanup or reactor-completion wait
allows that budget plus 6000 ms. These are configurable bounds, not measured throughput claims.

The selected-device production host helper now advertises manufacturer/model
`Blent`/`Blent`, version `1`, matching Android's accessory filter. The explicit
prototype helper path keeps its separate `Blent USB Probe` profile. Neither
profile grants authentication. Android's manifest opens MainActivity on accessory
attachment; T893 now connects Android UI/presentation and activity lifecycle to the new
entry point. Real display/touch, camera and audio remain T857–T860. No ADB-free
production-media support or physical-device acceptance is claimed by composition.

Validation and exact source/counter evidence are retained under
[T854 artifacts](artifacts/2026-10-10-t854/README.md). Synthetic KeyStore provider
tests exercise the loader boundary and signatures only; native AndroidKeyStore
isolation, physical USB and actual Binder-kill acceptance remain T855/T846.

## T892/T893 production Android pairing presentation

MainActivity now retains the application-owned `ConnectionBinding` through
`ConnectionOwner`, which captures only application context. Recreating, stopping
or destroying an Activity does not forget a still-owned USB connection. Android
hides the approval dialog while the Activity is stopped and presents the same
current choice when it resumes. Explicit Disconnect/Revoke retires the link;
individual media owners keep their own foreground/background consent policies.
Actual process death remains the containing native ownership boundary.

USB accessory intents start fresh selection/permission admission when no attempt
is active; they never consume token extras or reapply legacy ADB credentials.
Duplicate attachment intents do not replace an active attempt. Ordinary launcher
and token-delivery intents retain the existing ADB path. The settings sheet has a
separate USB section, and an approval dialog appears without opening settings.
It compares Computer with Computer and Tablet with Tablet across devices, resets
Remember for each exact displayed choice, and treats denial/dismissal explicitly.
The dialog scrolls on short windows. Camera/microphone permission stays separate.

T895 publishes restored peer metadata without prompting or granting TLS proof;
T896 publishes authenticated status only after verified proof, durable trust,
negotiation and I/O installation. Current-peer revoke waits for a durable trust
result. Disconnected saved-computer management is follow-up T897; user-editable
connection capacities/deadlines remain T898. The UI currently uses validated
T854 defaults. Production display/touch/audio/camera routing remains T857–T860.

[T893 evidence](artifacts/2026-10-10-t893/README.md): 711 offline connection/UI
regressions pass on applicable JVM/API 27/34 fixtures. All 139 scoped inventory
functions and 162 executable JVM methods meet individual 80% line coverage;
41 production complexity entries are at most 9. Physical production pairing,
revocation and AndroidKeyStore/accessory behavior remain T855/T846; none is
inferred from Robolectric, and no installed app was replaced for these checks.

## T894 production host pairing controls

The GUI now has a USB connection tab backed by T900 immutable snapshots and the
T901 Linux native client. T903 associates each queued command with an exact,
retained outcome. Device connection, corresponding Computer/Tablet fingerprint
comparison, explicit approve/deny/remember, disconnect and saved-peer revocation
are available. Stale tickets and reconnect lifetimes cannot replay choices;
background refresh cannot confirm a queued mutation. Retry waits for actual
client/native retirement, and terminal status with incomplete cleanup is visible.

Local acceptance uses sibling broker/helper binaries and private runtime/state
roots, resolved by the Linux adapter. Packaging remains deferred T861; Windows
ADB-free USB remains unsupported T902. Existing media/control protocols are not
yet routed over the connection. T855 retains joined production-device pairing
acceptance, followed by display/touch T857, camera T858 and audio T860/T859.

[T894 evidence](artifacts/2026-10-10-t894/README.md) retains 98 passing GUI unit
tests plus actual isolated GUI startup, native per-function counters for all new
functions and changed App wiring, stale-action/lifecycle widget tests and the two
draft fail-before-fix regressions. No device support is inferred from those tests.

## T905/T906 Android local channel ownership

The connection worker now composes the protocol runner with a bounded local
resource controller. Display owns control and video listeners as one exact lease.
Both listeners bind IPv4 loopback with OS-allocated, distinct ports before
Prepared; an invitation becomes visible only after current protocol admission.
No wire field selects an address or port. Other resources still require their
separate permission and native adapters and cannot acquire listeners here.
At T906 the capability mask stayed zero pending Activity composition. T907 now
advertises display and binds its routes on Android. Host integration remains T908;
joined production USB display acceptance remains T857.

Native bind, accept and close run on retained workers. Each channel has at most
one outstanding read and one write/flush, and one bounded producer block. Block
size is capped by the negotiated window and 64 KiB. Consumer writes borrow ring
storage until exact write/flush completion; the resource cannot return ownership
while a pump or native endpoint remains live. An idle accept begins its startup
deadline only on admission. Bind and active I/O deadlines remain absolute.

The controller drains at most 32 endpoint events per turn and scans four resource
slots. A failed channel cancels its resource and its sibling display channel;
resource quarantine does not terminate an otherwise live connection. Whole-link
shutdown requires every native owner to retire or the containing process to exit.
Partial construction, failed close and late failed stop retain uncertain ownership;
a timeout alone never grants a replacement. The worker's existing independent
10 ms default timer drives progress even without native wakeups.

[T906 evidence](artifacts/2026-10-10-t906/README.md) includes real JVM loopback
listeners and protocol/mux/pump traffic in both directions, bounded invalid-port
and lease-counter cases, late-construction/cancellation regressions, stale leases,
partial factory failure, sibling cleanup and per-method native counters. These
fixtures do not establish physical USB, Android Binder/Activity lifecycle or
rendered-frame/input acceptance. T855's production tablet pairing trial is deferred
by the owner's 2026-10-10 test-later decision; implementation continues locally.

## T909 private display lease publication

The worker now obtains routes through a runtime snapshot guarded by current
pairing approval, matching connection ticket and exact resource admission. Setup
metadata cannot publish media events. Private same-UID IPC carries a bounded
route and its exact closure, reusing the existing 4096-byte envelope and bounded
queues. Media payload remains in the connection process.

Presentation checks the worker-generation reference, authenticated peer ticket
and local resource-owner sequence. Equal counters from another process cannot
replay a route, and a late closure cannot clear a replacement. Whole-connection
admission cancellation publishes exact route revocation before native cleanup
waits. Native ownership and logical UI authority remain separate.

[Evidence](artifacts/2026-10-10-t909/README.md): 617 passing offline tests; all 157
scoped functions and 231 executable JVM methods meet individual 80% line coverage;
261 reviewed production/test functions have cyclomatic complexity at most 9.
T907 supplies Activity/session binding and enables Android display advertisement.
Physical pairing and joined display/input acceptance remain
T855/T857 and were not run during this implementation.

## T907 Android Activity display routing

Android now advertises display capability only. A private route reaches the
Activity adapter only with active authenticated presentation and a matching peer
ticket. The adapter translates it to the independent `DisplayEndpoint` contract;
`SessionCoordinator` owns route revisions and applies control/video together.
A route change invalidates both old destinations and pending control callbacks
before installing new credentials and locally allocated ports. Video waits for
the new control greeting's mode/codec. Installation failure clears both routes
and reports an error; revision exhaustion cannot wrap into old authority.

An explicit active USB selection with no admitted display clears the Activity's
old destination. Stopping an Activity cancels its route observation and stops
control/video while preserving the application-owned connection and independent
camera/audio owners. Resuming checks the latest route before reconnecting;
recreation creates fresh Activity bindings against the retained connection.
A revoked route cannot be restored by background resume or persisted ADB
credentials. Fresh legacy token delivery can select ADB after USB route revocation;
it cannot replace an available USB display. A newly opened Activity with no active
USB selection retains ordinary ADB startup. USB credentials never enter the
legacy preference store. Route revisions span backend changes within a session.

[Scoped validation](artifacts/2026-10-10-t907/README.md) retains API 27/34 lifecycle,
Activity recreation, private route/credential boundaries, stale callbacks, partial
installation and bounded invalid-input regressions. Host registration/reconnection
still belongs to T908. Real rendered frames and touch/mouse require T857; the
owner-deferred tablet pairing trial remains T855. Nothing was installed or reloaded
for this implementation.

## T910 portable host registration client

One client model owns one resource registration exchange per local IPC lifetime.
Request IDs 1 and 2 correlate registration and cancellation; broker-issued route
owner/revision counters disambiguate delayed statuses. Only exact current-route
Active can admit media. Closing or failure immediately revokes admission and cannot
be undone by a late Active response. Register exposure is recorded before native
I/O, so an interrupted operation cannot silently free uncertain remote ownership.

Pre-exposure cancellation and pre-ownership rejection prove no remote resource;
otherwise explicit Retired is required. Failed status, IPC loss, deadlines and
native worker exit alone do not prove broker endpoint retirement. The independent
Client contract also requires actual native worker retirement before releasing its
capacity. Native descriptors remain outside the portable model. T911 supplies the
Linux adapter; T912 provides retained session listeners before T908 integration.
[Validation](artifacts/2026-10-10-t910/README.md) covers bounded transitions and
invalid inputs with native per-function counters. No physical acceptance claimed.

## T911 retained Linux registration adapter

The process-owned factory uses authenticated local packet IPC, configurable
capacity (default 16, range 1–256), startup (10 s), admission (30 s) and cleanup
(5 s) deadlines. Deadlines must be positive and at most 60 s. More capacity retains
more workers/listeners; longer deadlines tolerate slower work while retaining its
resources. User-facing controls remain T898.

Each resource has one retained native thread and IPC lifetime. Exclusive IPv4
loopback listeners are validated using the existing adapter, sent atomically and
retained independently of caller handles. Logical close immediately revokes
admission and wakes Unregister; the IPC cleanup path stays open for exact broker
proof. Repeated close or malformed replies cannot extend cleanup deadlines.
Active idle routes have no arbitrary media-read deadline.

Capacity releases only after remote retirement (or proof of no registration) AND
actual native thread/runtime retirement. Failed IPC, timeout or panic retains
uncertain ownership/listeners while siblings progress. Lost IPC currently leaves
that slot quarantined for the application lifetime; no transparent replay occurs.
Safe recovery requires independent proof of the original broker's native death.

[Validation](artifacts/2026-10-10-t911/README.md) covers isolated broker/TLS traffic,
descriptor lifetime and bounded failures. T912/T908 still own session-listener
publication and host integration; T855/T857 physical acceptance remains pending.

## T912 session listener ownership

`Prepared::start_exported` is an explicit native TCP composition seam: it returns
the ordinary session runtime plus separately owned `RetainedListeners`. The
portable capture/input/attachment and lifecycle contracts do not gain OS handles.
The network adapter retains bound video/control capabilities and exports ordered
owned duplicates to registration adapters. Numeric ports describe their actual
bindings; an ephemeral bind now reports its assigned port rather than zero.

Ordinary session startup drops the extra export while its accept loops retain
their listeners. Registration startup retains its own duplicates through broker
retirement, independently of session-task retirement. The composition owner must
retire both domains before reusing a session; one domain's completion is not proof
for the other. Partial bind or capture startup failure drops all unpublished
listener owners before returning an error.

[Evidence](artifacts/2026-10-10-t912/README.md) retains fail-before-fix port reporting,
native descriptor lifetime, startup rollback and session lifecycle regressions.
T908 still joins authenticated peer discovery, session selection and registration.

## T916 registered display attachment authority

A `RegisteredRoute` binds exact credential, authenticated peer identity and local
generation under one lock. Credentials are required independently of legacy ADB
preferences. Only the composition owner holding exact current broker Active proof
may admit it. Stale route admission/close cannot alter its replacement, and native
counter exhaustion permanently closes the attachment rather than wrapping.

Close/drop invalidates old sockets and credential immediately. Same-peer renewal
retains negotiated geometry and desired display presence, allowing the configured
virtual display to survive a decoder/control route replacement; native cleanup
and fresh registration admission remain separate requirements. A different peer
resets geometry/presence. Whole-peer loss explicitly clears presence, while
absence or descriptive identity alone never restores tokenless authentication.

Explicit ADB fallback restores the configured legacy authentication policy with
fresh required credentials. Its composition owner must first retire the prior
native route/session ownership before reassigning a slot (T917/T908). Credential
revocation alone is not retirement proof. [Evidence](artifacts/2026-10-10-t916/README.md)
retains red/green concurrency, fallback and exhaustion regressions and native
per-function counters. Production monitor composition remains T908.

## T919 monitor retirement prerequisite

The existing monitor now retains each display slot after bounded logical session
stop until its returned native completion is actually complete. Route cleanup
runs before this final wait, and independent retirement jobs continue. The daemon
keeps its existing bounded shutdown behavior. [Regression and counters](artifacts/2026-10-10-t919/README.md)
prove blocked-native retention, sibling progress and final release. This closes
the pre-existing slot-reuse gap before shared ADB/USB admission under T917.
