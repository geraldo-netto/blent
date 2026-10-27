# Audio contracts and implementation

For setup and controls, read [audio sharing](audio.md). This reference specifies
transport, consent, native ownership and exact packet bounds. PCM means
uncompressed audio samples. A lease identifies one admitted resource; retirement
means its original worker has actually finished and released it.

```mermaid
flowchart LR
    A[Validated Start] --> B[Tablet consent and permission]
    B --> C[Authenticated audio connection]
    C --> D[Streaming]
    D --> E[Stop or failure]
    E --> F[Wait for original workers and route]
    F --> G[Allow replacement Start]
```

Microphone and speakers follow this lifecycle independently. A timeout or closed
socket cannot skip the cleanup wait. ADB uses the [temporary mapping contract](adb-media-mappings.md).
Direct USB supplies its separate authenticated transport adapter.

## Transport and validation boundaries

`blent_config::audio` owns portable policy. `AudioOptions` selects ADB by default
or a full lowercase USB fingerprint independently per direction. USB peer and ADB
serial are mutually exclusive; shortened, uppercase or invalid peers fail.
Linux `accessory-transport` builds implement both USB directions without falling
back to ADB. Unsupported saved choices remain visible and unchanged by rendering.
Apply is passive; Start validates the full profile and selected capability before
constructing a backend. Failed construction retains no backend.

Camera/audio share `connection::media` invitation/monitor/close contracts and the
host `media_connection::Bridge<P>`, typed `Connection<P>`/`Invitation<P>` and
`MediaRegistration` owner. Only the Linux adapter builds discovery/registration
process settings and transfers the retained listener descriptor. Profiles keep
direction, processing, buffer and background preferences; transport selection is
outside the wire schema. Clock negotiation stays in the PCM bootstrap.

`media_request` checks resource/profile consistency and fixed lowercase-hex
credentials. Native setup independently checks that the bootstrap token matches
the original request. The helper validates the entire profile and strict
64-character lowercase-hex token before slicing a label or spawning; short or
multibyte tokens and invalid buffering cannot create a helper. Existing camera
boundary errors remain unchanged.

Each USB session retains its originally selected source/route through consent,
streaming and cleanup. Source loss ends that session; it never selects a replacement.
Stopping before startup creates no helper/mapping. Microphone and speaker keep
separate invitations, helpers, credentials and completion proofs. One stalled
direction cannot free its admission or revoke a sibling. Invitation success means
transport acceptance, not native or remote retirement.

## Entropy and independent capability contracts

Native Rust applications enable `native-entropy` (default and `platform`) and use
system randomness. Policy-only builds can compile without OS randomness, including
`wasm32-unknown-unknown`, but default Start fails closed. An injected
`AudioSession::with_entropy` CSPRNG must fill the entire credential or fail without
changing state/generation. Test adapters do not change production entropy.

The selected product requirements are [microphone/shared audio](reviews/2026-09-29-audio-input.md)
and [tablet speakers](reviews/2026-09-29-audio-output.md). Their planned native work
must be distinguished from implemented behavior and measured acceptance below.

## Android consent and original consumer identity

Camera and audio share `MediaConsentPolicy<O>` and `MediaConsentObservation<O>`
through independent `MediaAccessOffer`, authority and control interfaces. Local
intent belongs to one exact lifetime across its permission/admission publications.
Replacement or source withdrawal clears it. Foreground Start, fresh permission
and requested/local background consent are separate conditions. SDK completion
cannot grant a replacement or substitute for a fresh permission sample.
Observation failure affects only its policy and cannot prove retirement.

`AudioAccessOffer` adds validated direction/preferences without credentials or
native handles. Speakers never request microphone SDK permission.
`AudioConsumerTarget` carries endpoint metadata and `MediaConsumerAuthority`.
`AudioBinding` retains the original claim before admission; only actual ResourceLane
retirement or proof that nothing started releases it. Selected USB ownership
suppresses unrelated ADB invitations/prompts. Withdrawal needs fresh selection;
matching metadata never substitutes for original authority.

`MediaConsumerRun` checks fresh permission, original claim and cancellation before
and after lazy native creation. A worker created during permission change stays
owned until retirement. Failed cleanup quarantines that direction. Binding rejects
invalid direction/preferences before acquiring native resources.

`LocalSpeakerOwner` requires local Start, requested/local background consent and
original lease before preparing/admitting its endpoint. `LocalAudioOffer` has no
credentials; `LocalAudioRoute` publishes only the allocated loopback port and
validated profile. `LocalMediaResource` joins actual endpoint/pump and possible
external-consumer completion for camera/speakers. Logical closure, source loss or
deadlines cannot release it. Uncertain cleanup is quarantined without retrying
proof or releasing capacity. The bounded bidirectional pump preserves bootstrap,
clock negotiation and PCM, forwards control promptly, and accepts the exact late
consumer completion after connection Stop. Microphone uses the shared leased adapter.

## Android commands and worker publication

Private commands carry the exact microphone/speaker lease, `MediaLocalConsent`
and a separate actual-consumer completion command. Offered/admitted/closed events
contain bounded metadata, never PCM or OS handles over Binder. Flat codecs reuse
management validation and reject malformed, truncated or trailing records while
preserving display/camera tags. Decoded data alone grants no native authority;
the retained original worker validates it.

The actor routes choice/completion to the original worker. Publication checks the
requested direction, authenticated ticket and current resource. Stop suppresses
new controls but allows exact late completion. Stale, shown-only, foreign or
completed claims reject. Completion queues retry boundedly; failed delivery proves
neither retirement nor permission to replace.

`ConnectionMediaConsumerOwner`, relay, session, presentation and change-only
publication are shared through independent contracts and resource adapters. Each
direction holds one unresolved owner and scalar watermark. `ConnectionBinding`
exposes typed original targets; queued UI actions retain exact publication identity.
`ConnectionMediaAccess`, `MediaConsumerSelection` and `MediaConsumerObservation`
require a matching admitted route/consumer and preserve same-owner local intent.
Queued callbacks cannot consent to a replacement.

`AudioOwner` composes separate policies for microphone/speakers. Every USB offer
needs local Start. Saved background settings alone grant neither Start nor
per-offer consent; requested background audio also requires that direction's
setting. Stop/source loss or foreground loss without background consent withdraws
only that direction. The current Activity owns its attachment; an older Activity
cannot detach its replacement. ADB settings/invitations remain available when no
USB offer is selected. Actual worker cleanup is independent of UI closure.

## Native composition and admission

Production Android advertises display, camera, microphone and speakers (mask 15).
After consent, speakers use isolated `SpeakerWorkerService`; microphone uses
`MicrophoneWorkerService`. Both keep authenticated PCM negotiation, processing
override, optional clocks, buffering and background choice. Microphone additionally
requires its original SDK permission and retains gain/input preferences; speakers
retain volume/route/low-latency preferences. USB withdrawal is independent of ADB.

Controller admission observes portable completion proofs independently of status
or backend Result. Per direction, a fixed pair tracks original USB owner and native
helper. Linux records each proof immediately after creation, before publication.
Closed descriptors, cancellation, helper execution exit, timeout or backend return
(success/error/panic) cannot release Start while actual retirement is pending.
Stopping also retains control-worker completion through bounded Drop/shutdown.
Other controls remain independent. Settled slots are reused without unbounded
history or automatic restart. The pipeline drops its original connection owner
before awaiting proof, avoiding a self-held barrier; native cleanup owners remain.
The CLI requests cleanup even if Ctrl-C registration fails, then awaits it.

## Evidence and limits

Private tests use actual TLS, descriptor registration, management/multiplexing,
loopback PCM and Android services/Binder with synthetic audio devices and controlled
process-death notifications. They cover exact samples/clocks, denied/stale consent,
cancellation, native failure/death, blocked reads/rendering, held leases, delayed
retirement and sibling independence. These establish protocol/ownership behavior;
they do not certify hardware, acoustics or physical process/accessory separation.

| Boundary | Permanent evidence |
| --- | --- |
| Shared transport/host registration | [T977](reviews/artifacts/2026-10-11-t977/README.md), T991 contracts |
| Consent / consumer / local route | [T982](reviews/artifacts/2026-10-11-t982/README.md), [T983](reviews/artifacts/2026-10-11-t983/README.md), [T984](reviews/artifacts/2026-10-11-t984/README.md) |
| IPC / original worker / Activity | [T988](reviews/artifacts/2026-10-11-t988/README.md), [T989](reviews/artifacts/2026-10-11-t989/README.md), [T985](reviews/artifacts/2026-10-11-t985/README.md) |
| Joined Android speakers / microphone | [T978](reviews/artifacts/2026-10-11-t978/README.md), [T998](reviews/artifacts/2026-10-11-t998/README.md); API 27/34 synthetic microphone fixtures |
| Linux speakers / microphone | [T992](reviews/artifacts/2026-10-11-t992/README.md), [T999](reviews/artifacts/2026-10-11-t999/README.md); T994 cancellation ordering and T1000 duplex isolation |
| Host controls / retirement / helper validation | [T979](reviews/artifacts/2026-10-11-t979/README.md), [T995](reviews/artifacts/2026-10-11-t995/README.md), [T993](reviews/artifacts/2026-10-11-t993/README.md) |

Selected API 36 RugKing Pad 2 Pro [speaker acceptance T860](reviews/artifacts/2026-10-20-media-acceptance/T860/README.md)
and [microphone/duplex acceptance T859](reviews/artifacts/2026-10-20-media-acceptance/T859/README.md)
passed. [T1237](reviews/artifacts/2026-10-21-t1237/README.md) records real microphone
PCM and owner-confirmed speaker tones with debugging disabled, fresh consent and
software reconnect. [T1238](reviews/artifacts/2026-10-21-media-acceptance/T1238/README.md)
records original workers retiring after hot reconnect and before debugging was
restored; it does not explain earlier uninstrumented Stop reports. Raw microphone
processing was unsupported. Sustained drift, acoustic echo cancellation, other
providers/devices and broader joined T846 acceptance remain unqualified.
Historical T835/T902 reports distinguish SDK/native-counter and unavailable Windows
validation from these selected-device results; interfaces alone are not support.

## Ownership and adapter integration

Keep one `AudioSession` and native `AudioBackend` owner per direction throughout
an attachment's lifetime. Microphone and speakers have separate state, credentials,
queues, sequence tracking and retirement. A failure in one does not stop the other.
Android's process audio owner retains one exact foreground attachment. A stopped
older Activity cannot detach a replacement or stop its microphone/speaker runs.
Microphone SDK results retain an independent original permission-request identity;
equal or reused endpoint data cannot grant or deny a replacement invitation.
The SDK adapter retains its actual outstanding request slot until result or
Activity destruction. Foreground detach stops only foreground audio; explicitly
permitted background runs keep their service and actual native retirement owner.
These lifecycle fixes also govern the implemented USB speaker/microphone
adapters. Selected-device physical speaker T860 and microphone/duplex T859
acceptance pass separately; other devices/providers remain unqualified.

The native implementation must intersect both endpoints' current capabilities before
Start. A supported direction means its 48 kHz s16le channel layout is supported.
Speech capability means processing can be requested, not that AEC works effectively.
Raw and background support are separate capabilities; neither is silently assumed.

1. Explicit user Start calls `AudioSession::start` with validated profile, current
   capabilities, authenticated-tablet status and local monotonic milliseconds.
   It returns a fresh 256-bit CSPRNG credential and increasing generation in an
   opaque `AudioGrant`. Failure leaves the session unchanged. Settings Apply,
   application launch and reconnect must never call Start implicitly.
2. Deliver `AudioGrant::hello()` only over the existing authenticated control path.
   Start the adapter asynchronously; it owns devices, paths, permissions, task
   cancellation and IO deadlines. The trait enqueues work rather than waiting on
   UI/native callback threads. The native adapters are described below.
3. A fixed handshake must match the grant before `connected` admits Streaming.
   The startup deadline is 5 seconds. Failed negotiation cannot acquire ownership
   or extend the deadline. Do not log grants/handshakes or persist credentials.
4. Use one authenticated connection per direction. Validate the fixed frame header
   with `FrameReader::payload_bytes` before allocating or reading its payload;
   `decode`/`AudioSession::receive` validates the complete frame atomically. A native
   reader may maintain its own FrameReader obtained from the grant alongside the
   session's independent validator. Never reset it between packets.
5. Poll `tick` from the owning worker even without incoming bytes. Streaming has a
   250 ms complete-frame deadline, refreshed only by accepted frames. Deadline or
   clock regression closes admission and clears PCM. It returns the generation
   whose adapter must be stopped. Partial socket reads cannot keep a session alive.
6. User Stop calls `stop`. Permission/focus loss, disconnect and asynchronous native
   failure call generation-bound `cancel`; late callbacks cannot cancel replacements.
   `receive` errors on the current generation also enter Stopping; the worker must
   retire its adapter when observing this state. Native retirement completes off
   the UI thread, followed by `retired(generation)`. Keep Stopping if retirement is
   stuck; do not release the ownership barrier merely because a timer expires.
7. Replacement Start is rejected until retirement completes. Reconnect does not
   resume audio. Stale render calls emit silence and cannot stop a newer session.
   Reset the whole owner only after all its native workers have retired.

IO workers must impose cancellable deadlines for handshake, header, payload and
writes. Device callbacks must use adapter-owned rings, without socket IO or waiting.
This domain layer performs no IO and cannot enforce an adapter's native shutdown.
T718/T719 retain tests for partial reads/writes, cancellation and native retirement.

## Versioned wire format

The current grant is `BLAUD002` (PCM plus a native-counter prefix).
PCM encoding and 10 ms block duration are fixed. Obsolete `BLAUD001` grants
and frames without the counter prefix are rejected.
Integer metadata is big endian; PCM samples are signed **little-endian** 16-bit.
Microphone uses direction 1/mono; speakers use direction 2/stereo, both 48,000 Hz.
The 92-byte handshake must exactly match the authorized session:

| Byte offset | Length | Value |
| --- | ---: | --- |
| 0 | 8 | `BLAUD002` |
| 8 | 64 | Fresh credential, lowercase hexadecimal ASCII |
| 72 | 8 | Nonzero session generation |
| 80 | 1 | Direction: 1 microphone, 2 speakers |
| 81 | 1 | Channels: 1 microphone, 2 speakers |
| 82 | 4 | Sample rate: 48000 |
| 86 | 2 | Frames per block: 480 |
| 88 | 1 | Processing: 1 speech, 2 raw |
| 89 | 1 | Background consent: 0 or 1 |
| 90 | 2 | Queue target: 20–200 ms, steps of 10 |

Each packet starts with a 28-byte header:

| Byte offset | Length | Value |
| --- | ---: | --- |
| 0 | 8 | Authorized generation |
| 8 | 8 | Sequence: starts at zero, strictly increases, `u64::MAX` rejected |
| 16 | 8 | Sender monotonic timestamp in microseconds |
| 24 | 2 | Payload bytes: 984 microphone, 1944 speakers |
| 26 | 1 | Authorized direction |
| 27 | 1 | Reserved, must be zero |
| 28 | 24 | Native clock prefix, always present |
| 52 | 960/1920 | Exactly one interleaved PCM block |

The prefix contains three big-endian unsigned 64-bit values: native epoch,
frame position normalized to 48 kHz, and native monotonic nanoseconds. All-zero
bytes explicitly mean unavailable. Otherwise epoch/time must be positive and all
three fields must fit a signed 64-bit value for the Android contract. Invalid
counters reject the complete packet without advancing receiver sequence state.

Timestamps strictly increase after the first frame. No subtraction of host and
tablet absolute clocks is used. Replayed/reordered frames, changed direction or
generation, unknown protocol, wrong format, malformed lengths, truncation and
trailing data are rejected. A sequence gap is accepted with a discontinuity and
flushes queued audio. Parsing uses fixed bounded storage; maximum packet size is
1972 bytes (1012 for microphone). Authentication is a connection credential check, not encryption or a
per-packet MAC. ADB carries this wire over its authorized connection. Direct USB supplies
its separate authenticated transport adapter; the PCM framing itself adds no
encryption or per-packet authentication.

## Buffering, drift and silence

`AudioProfile` defaults to speech, foreground-only and a 40 ms application queue
target. The target accepts 20–200 ms in 10 ms steps. Lower values reduce startup
buffering but tolerate less jitter. Higher values add lag. This budget does not
include Android, PipeWire, device or transport buffering and is not a measured
end-to-end latency promise.

`PcmQueue` preallocates space for twenty complete 10 ms blocks, independent of the
requested prefill target. Rendering waits for that target after Start/underflow.
Overflow drops the oldest block (including any unconsumed remainder), preserving
channel alignment and reporting discontinuity. Blocks older than 200 ms on the
receiver's local clock are discarded. Underflow returns an entire silent block;
it never repeats old speech. Stop clears pending samples immediately. Queue and
render calls reject backwards clocks and incompatible sizes/channel layouts.

`adjust_drift` accepts native source/destination frame-counter deltas normalized
to the same duration. Rust and Android form independent two-to-five-second native
windows and compare `source_frames × destination_ns` against
`destination_frames × source_ns`. They never subtract absolute endpoint clocks or
estimate a native rate from packet arrivals. Completed windows expire after three
seconds on the receiver's local clock; repeated timestamps cannot refresh them.

PipeWire reads the graph driver's SPA position, rate and monotonic timestamp;
Android reads AudioRecord/AudioTrack timestamps. These are native pipeline/graph
observations, not guaranteed physical hardware oscillator measurements. Clock
availability and accuracy depend on the route/backend. Epoch, frame/time reset,
stalls, invalid windows and excess drift reset correction. Missing timestamps
leave ordinary bounded playback at nominal rate and report drift as unmeasured.

Correction is limited to ±1000 ppm using the exact ratio before integer rounding.
A fixed-point linear interpolator preserves channels and fractional phase across
blocks. The Rust render callback allocates no memory and performs no IO; Android
interpolation uses bounded arrays on its IO worker. Focus flushes and AudioTrack
replacement advance the playback epoch. Existing queue age, underflow and
retirement bounds remain in force. Microphone correction/unavailable status is
logged numerically outside the native callback; Android speaker status reports
it alongside native buffer/route diagnostics. No speech or PCM content is logged.

The [native integration evidence](reviews/2026-09-30-audio-clock.md) covers synthetic
rates, adapters and private PipeWire graphs. This is not an acoustic latency,
fidelity, physical duplex or AEC effectiveness claim.

## Validation

[September 29 evidence](reviews/2026-09-29-audio-foundation.md) records permanent
normal-suite tests, scoped per-function native Rust coverage and boundary findings.
[T718 evidence](reviews/2026-09-29-audio-microphone.md) records native source/transport
regressions and bounded physical checks. [T719 evidence](reviews/2026-09-29-audio-speakers.md)
records private sink/transport tests and connected-tablet playback/lifecycle.
These September reports record their then-outstanding follow-ups. Later selected-device
duplex/USB acceptance is linked above; sustained drift and measured acoustic
effectiveness/delay remain unqualified.

## Linux/Android microphone integration (T718)

For user controls, dependencies and processing choices, see [audio sharing](audio.md).
The microphone source belongs to one session. Permission, native errors, Android
silencing, observed route changes and transport loss govern its shutdown.

The DUMP-protected Android receiver admits an invitation over the authorized ADB
connection. The authenticated 76-byte bootstrap is `BLAUREQ2`; obsolete
`BLAUREQ1` requests are rejected before audio admission. It carries the invitation
credential (64 hex bytes), capability bits (speech/raw/background), AEC-enabled
flag, direction and effective processing. Human consent has a separate 90-second
budget; it does not consume native startup's five seconds. The host echoes the
invitation credential followed by the 92-byte grant after PipeWire readiness.
Android gives that complete nonce/grant one five-second deadline; partial reads
cannot renew it ([T724 regression evidence](reviews/artifacts/2026-09-29-audio-grant-deadline/README.md)).
Native microphone input preserves sequence-gap discontinuities; the callback uses
only its bounded queue, expires old data and emits silence on underflow. It never
waits on the transport.

Native device-counter integration uses the mandatory prefix. Unavailable counters
use its all-zero encoding; this keeps audio available without claiming measured
drift. Deploy matching host/Android builds; there is no protocol downgrade.
The original T720 follow-up does not establish measured physical duplex
drift/latency or general route acceptance; selected-device results are scoped above.

The private Linux helper accepts exactly three arguments: queue target, owned node
name and direction (`microphone` or `speakers`). Its fixed IPC blocks contain a
one-byte discontinuity flag, the 24-byte clock prefix and PCM: 985 bytes for
microphone, 1945 for speakers. The obsolete optional `clocked` argument and
implicit microphone direction are rejected.

## Linux/Android speaker integration (T719)

Each speaker session owns one PipeWire `Audio/Sink`, leaving defaults and unrelated
application routes unchanged. Android reports actual route, native buffer/played
frames and underruns separately from the application queue target. Native buffer
minima can exceed that target. An observed route change requires explicit Start;
a dead AudioTrack can be recreated within the same authorization, discarding the
interrupted block. Speech requests communication playback; Raw requests media.
Neither proves effective echo cancellation.

Low-latency mode is opt-in and may be denied; diagnostics report the granted mode.
It does not change the native buffer request. Muted tests found about 67 ms less
counter-estimated queued audio, not measured acoustic delay.

Focus denial prevents playback. Temporary loss (including duck requests) pauses
and flushes; regain can resume only the same session. Paused rendering waits for
focus without timestamp polling, and Stop/transport completion cancels that wait.
Permanent focus loss stops playback. Foreground-only playback stops when hidden;
explicit background consent owns its own mediaPlayback service and Stop notice.
Changing tablet settings stops that direction. Launch/reconnect never resumes it.

The PipeWire callback copies bounded stereo chunks into its queue without pipe IO
or waiting for a mutex. It rejects corrupt/unknown chunk flags, treats EMPTY as
silence, and honors SPA's modulo offsets and clamped sizes. Incomplete chunks
expire after a 200 ms capture stall. A
worker dequeues whole blocks without adding another prefill, emits silence on
underflow, and paces transfer at 10 ms. Android validates each complete frame within
one 250 ms deadline, keeps at most twenty blocks, and applies the requested prefill
after Start/underflow. Gaps and overflow discard stale samples. AudioTrack short
writes share one 250 ms complete-write budget; partial progress cannot renew it.
Stop, transport loss or PipeWire server loss removes the owned sink and releases
playback. Native counter correction follows the policy above. These limits bound
application backlog; they do not establish measured acoustic latency.

## Host staging and identity-rate rendering

Each microphone connection retains one fixed native packet buffer; speaker capture
and wire serialization reuse bounded storage. Validation still commits sequence
state only after the complete packet and optional clock pass. Partial I/O keeps the
existing deadline/connection-retirement behavior. At exact zero drift and phase,
the worker queue bulk-copies PCM slices; fractional resampling remains unchanged.
[T818 stage measurements](reviews/2026-10-10-audio-staging.md) show lower allocation
traffic and processing time, without claiming lower acoustic latency.
