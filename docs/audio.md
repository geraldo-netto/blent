# Audio foundation

T717 implements portable Rust audio policy in `blent_config::audio`. T718 adds
Linux/Android microphone transport, native source and controls described below.
T719 adds the selectable speaker sink and Android playback. T720 integrates native
device-clock correction; physical duplex/acoustic acceptance requires a device window.
Unimplemented backend capabilities remain unsupported. No system default device changes or automatic audio startup occur.
The audio CLI requests stop and awaits its original pipeline after Ctrl-C or
SIGTERM, including a failed Ctrl-C registration, before returning the result.

Native Rust applications enable the shared crate's `native-entropy` feature
(included by default and by `platform`). A policy-only build compiles without
OS randomness, including on `wasm32-unknown-unknown`; its default audio Start
fails closed. A backend can supply a CSPRNG through `AudioSession::with_entropy`.
That adapter must fill the complete credential buffer or return an error; a
failed fill leaves session state and generation unchanged. Test adapters do not
change the native applications' use of system randomness.

The selected product contract is [microphone/shared audio](reviews/2026-09-29-audio-input.md)
and [tablet speakers](reviews/2026-09-29-audio-output.md). The following describes
implemented shared behavior; those reviews also contain planned native behavior.

## Portable connection selection (T977)

`AudioOptions` and `blent audio --usb-peer <full-tablet-fingerprint>` now accept an
explicit USB target independently for microphone and speakers. It conflicts with
`--serial`; invalid, shortened or uppercase fingerprints fail validation.
Existing settings and commands retain ADB selection by default. Saving a target
starts no media or connection. The Linux accessory build now implements USB speaker PCM under T992. USB
microphone remains unavailable and rejects before ADB, helper or device work;
production selector controls are implemented under T979. Android leased
speaker playback is composed under T978; microphone integration remains T859. These
contracts do not establish physical USB audio support.

Camera and audio use shared platform-independent `connection::media` transport
and generic invitation/monitor/close interfaces. Profiles preserve direction,
processing, buffer and background preferences; transport selectors stay outside
the wire schema. Audio clock negotiation remains its authenticated PCM bootstrap.
Invitation success is transport acceptance only; cancellation, timeout or a
failed close does not prove native/remote retirement. Adapters retain original
ownership independently. [Offline contract and native fixtures](reviews/artifacts/2026-10-11-t977/README.md)
qualify this boundary; actual USB speaker/microphone acceptance remains T860/T859.

## Shared Android consent policy (T982)

Android camera and audio now share `MediaConsentPolicy<O>` and
`MediaConsentObservation<O>` through independent `MediaAccessOffer`, authority and
control interfaces. One exact original lifetime carries local intent across its
own permission/admission publications; replacement or source withdrawal clears
it. Foreground Start, fresh permission and explicit requested/local background
consent are separate conditions. SDK completion never grants a replacement or
substitutes for a fresh permission sample. Observation failure affects only its
own policy and never proves native worker or endpoint retirement.

Camera and selected audio production callers use this shared policy. `AudioAccessOffer` adds validated
microphone/speaker metadata without transport credentials or native handles;
speaker offers cannot request microphone SDK permission. Independent audio
properties qualify this policy for the leased USB adapters. Android advertises implemented speaker routing/playback under T978;
Linux speaker PCM is implemented under T992; production host controls are implemented under T979. Android microphone composition is T998; Linux uplink is implemented under T999; physical acceptance remains T859. Legacy audio Activity/permission behavior
remains as described below. Camera behavior and permanent regressions are
preserved. [Policy and native counter evidence](reviews/artifacts/2026-10-11-t982/README.md).

## Shared Android consumer ownership (T983)

`AudioConsumerTarget` carries validated endpoint metadata and an independent
`MediaConsumerAuthority`. AudioBinding retains an exact claim before admission;
only actual ResourceLane retirement or a no-start proof releases it. Selected
external ownership suppresses unrelated ADB invitations and legacy SDK prompts.
Withdrawal does not revive an old invitation; a fresh explicit selection is
required. Equal endpoint metadata never substitutes for original authority.

Camera and both legacy/selected audio targets share `MediaConsumerRun`: fresh permission,
original claim and cancellation checks surround lazy native acquisition. A worker
created during a permission change remains owned until its actual retirement.
Failed cleanup quarantines its direction while the other direction remains
independent. Binding construction rejects invalid direction/preferences before
native ownership. These are adapter foundations; production USB speaker playback is composed under T978, with
host PCM transport is implemented under T992, with production controls under T979; physical acceptance T860 remains pending. T986 also rechecks legacy permission before and after lazy native creation;
queued permission loss cannot acquire a native worker. Created workers remain
owned through real retirement. [Regression and native counter evidence](reviews/artifacts/2026-10-11-t983/README.md).

## Android local leased speaker routing (T984)

`LocalSpeakerOwner` now implements pending local Start, requested/local background
consent and original lease checks before endpoint preparation and admission.
`LocalAudioOffer` contains no credentials; `LocalAudioRoute` publishes only a
locally allocated loopback port with the validated original profile. The shared
`LocalMediaResource` joins actual endpoint/pump and possible external consumer
completion for both camera and speakers. Logical closure, source loss and deadline
warnings never release that barrier. An uncertain cleanup publication quarantines
its resource without retrying proof or releasing capacity.

Speaker channels use the existing bounded bidirectional local pump, preserving
bootstrap, clock negotiation and PCM bytes. Control forwarding remains prompt;
a stopped connection still accepts its exact late consumer completion. Android
production composes the speaker authority under T989/T985/T978; physical
acceptance remains T860. Microphone remains unavailable in this adapter until T859.
[Private protocol/lifecycle and native counter evidence](reviews/artifacts/2026-10-11-t984/README.md).

## Private Android audio IPC contracts (T988)

Private connection commands now carry an exact microphone/speaker lease and
`MediaLocalConsent`, plus a separate actual-consumer completion command. Offered,
admitted and closed audio events contain bounded metadata only. The flat codecs
reuse management profile validation, reject malformed/truncated/trailing records,
and preserve existing display/camera tags. PCM and OS handles do not travel over
Binder. A decoded lease or completion command provides no native authority by
itself; original retained workers must validate it.

These schema foundations do not enable audio: T989 routes commands only through
a provided audio owner matching the exact original ticket/resource. UI consent and joined playback/capability are implemented under T985/T978.
[Contract properties and native counters](reviews/artifacts/2026-10-11-t988/README.md).

## Original Android audio worker publication (T989)

The actor and accessory admission now forward audio choice/completion to the
retained original worker. Runtime projections validate the requested direction,
original authenticated ticket and current resource before publishing a route.
Logical Stop suppresses new controls while still allowing exact late consumer
completion; stale, shown, foreign and already completed claims are rejected.
Private completion queues may retry boundedly, and failed delivery never proves
native retirement or authorizes replacement.

`ConnectionMediaConsumerOwner`, completion relay, session, presentation and
change-only publication are shared by camera/audio through independent local
metadata/command contracts and explicit resource adapters. Each direction holds
one unresolved owner and a scalar watermark. Matching metadata cannot alias a
replacement worker or revoke a sibling. `ConnectionBinding` now exposes typed
microphone/speaker targets with original authority; its UI actions retain the
exact current publication identity. Camera contracts and regressions remain.

Android production composition advertises speakers with display/camera under
T978. Microphone remains unavailable until T859; physical speaker acceptance
remains T860. Local consent/Activity controls are implemented under T985.
[Private worker/lifecycle regressions and native counters](reviews/artifacts/2026-10-11-t989/README.md).

## Original Android audio consent and Activity controls (T985)

`ConnectionMediaAccess` projects exact original peer/lease/publication ownership
for camera and audio through typed metadata adapters. Only an admitted route with
its matching consumer target can be selected. Queued UI callbacks cannot consent
to a replacement, and same-owner admission retains fresh local intent.
`MediaConsumerSelection` and `MediaConsumerObservation` share stable target
selection and source-withdrawal handling across camera/audio.

`AudioOwner` composes separate consent and consumer observations for microphone
and speakers. Each USB offer requires local Start; persisted background settings
alone grant neither Start nor per-offer background consent. Requested background
audio additionally requires the existing direction's background setting. Stop,
source loss and foreground loss without explicit background consent withdraw
only that direction. Android SDK completion remains bound to the original prompt;
speaker offers never request microphone permission. The exact foreground Activity
attachment owns both policies; an older Activity cannot detach its replacement.
Actual audio worker cleanup remains independent of logical Stop and UI lifecycle.

The passive audio settings and legacy invitation/SDK behavior remain available
when no USB offer is selected. Joined Android speaker composition uses these controls under T978; host PCM transport is implemented under T992; controls are implemented under T979. Android microphone composition is T998; Linux uplink is implemented under T999; physical acceptance remains T859.
[Permanent UI/lifecycle/projection tests and native counters](reviews/artifacts/2026-10-11-t985/README.md)
are synthetic evidence; physical qualification remains postponed under T860.

## Joined Android leased USB speaker playback (T978)

Production Android connection composition advertises display, camera, microphone
and speakers (mask 15), with explicit original session media authority. Microphone
consent/uplink is implemented under T996–T998; host microphone selection is implemented under T999. Speaker offers follow the T985 local consent path before allocating
an endpoint, then select an original target retained through `AudioBinding` and
the isolated `SpeakerWorkerService`. The existing authenticated PCM bootstrap
negotiates the tablet processing preference and optional native-clock format;
buffer, background, volume, built-in route and low-latency preferences are retained.
USB audio uses explicit lease withdrawal, independently of legacy invitations.

Permanent joined tests use real private connection/media services and Binder,
management/mux routing, loopback TCP, clocked grants/PCM and the audio renderer.
They cover denial, cancelled pending consent, stale publications, failed native
open, worker death, delayed native cleanup, delayed process-death proof and a
blocked renderer. Speaker closure leaves the original connection and display
available. Neither logical Stop, socket EOF, native execution end nor a deadline
alone proves actual original consumer retirement.

The fixture's audio device and process-death notifications are synthetic. It
establishes joined protocol and ownership behavior, not acoustic performance or
physical process/accessory qualification. Native speaker-only acceptance remains
T860; Linux PCM composition remains T992 and controls T979. [Joined tests and individual native counters](reviews/artifacts/2026-10-11-t978/README.md).

## Joined Android leased USB microphone capture (T996–T998)

Microphone offers require explicit per-offer Start and a fresh original permission
sample before endpoint preparation. The retained opaque SDK prompt routes through
the exact original worker/publication; denied, stale, withdrawn or unproven grants
cannot start capture. Shared consent state and native audio ownership preserve
independent speaker behavior. After admission, the exact target selects the isolated
`MicrophoneWorkerService` and existing authenticated mono PCM capture path.
Tablet processing override, gain, built-in input selection, buffer/background
preferences and optional native clock metadata remain in their existing adapters.
Requested background capture needs both host request and explicit local consent.

Permanent joined tests on Android API 27/34 use real private services/Binder,
management/mux and loopback TCP with synthetic microphone hardware and controlled
process-death notifications. They verify clocked and unclocked PCM, grant before
sampling, permission denial/loss, pending Stop/source loss, stale SDK callbacks,
background choices, native failures, worker death and blocked actual reads.
Display control continues while microphone cleanup is held; endpoint closure,
execution Ended and deadlines cannot prove actual worker/consumer retirement.
No acoustic, hardware/provider or actual process separation acceptance is claimed.
Linux uplink is implemented under T999; automated duplex isolation T1000 and postponed physical
acceptance T859/T846 remain explicit. [Offline native evidence](reviews/artifacts/2026-10-11-t998/README.md).

## Shared original USB media connection adapters (T991)

Host `media_connection::Bridge<P>` shares original invitation supervision and
retained cleanup across typed camera/audio profiles using the independent
`Connection<P>`/`Invitation<P>` contract and `MediaRegistration` owner. Only the
Linux adapter constructs discovery/registration process settings and passes the
retained listener descriptor. Camera retains its typed request adapter and its
existing wire behavior; this extraction does not enable native USB audio.

`media_request` validates explicit USB selection, resource/profile consistency
and strict fixed lowercase-hex credentials before registration. The native
constructor also checks that its bootstrap token describes the exact original
request credential, before native setup. Camera and audio share byte-format
validation, while existing camera boundary error semantics remain intact.

Permanent camera and typed audio properties preserve original requests,
replacement/replayed invitation rejection, pending cancellation, independent
monitor/close and held actual retirement. The private mutual-TLS camera fixture
still carries unchanged framing and feedback through the native descriptor path.
Native counters cover the default and accessory configurations; Windows runtime
validation remains unavailable under T902. Speaker PCM integration is T992,
controls T979 and postponed physical acceptance T860.

## Linux leased USB speaker PCM (T992)

The Linux accessory build supports explicit speaker selection through
`blent audio --direction speakers --usb-peer <full-tablet-fingerprint>`.
The complete options/profile and original credential are validated before native
registration. ADB remains the default and uses its existing owned reverse mapping;
USB never falls back to ADB. Stopping before startup creates no helper or mapping.
Linux USB microphone is implemented under T999 with Android consent/uplink T998. Physical qualification remains T859.

Both transports use the independent typed Connection/Invitation port. USB retains
the original selected source and route through generic media ownership. Pending
consent and streaming monitor that original lifetime; loss retires this session
without selecting a replacement. Authenticated bootstrap preserves tablet
processing override, buffering/background choices and optional native clocking.
The Linux process adapter still creates only this session's owned speaker sink.
No system default or unrelated application route changes.

Permanent fixtures join real private mutual TLS, broker descriptor registration,
management/mux channels, actual loopback PCM and synthetic native helpers. They
validate stereo samples, processing override and native clock fields, pending
Stop, source loss, failed/stalled helpers, held native leases and delayed remote
retirement. T994 fixes an obsolete local cancellation race revealed by this join.
Helper and original-route cleanup retain their own actual proof after deadlines.
T995 keeps per-direction controller admission occupied while that proof is
pending. No physical/acoustic qualification follows from these
fixtures. Production GUI selection is implemented under T979; postponed acceptance T860 and
SDK-wide native-audio validation T835 remain explicit.
[Offline tests and individual native counters](reviews/artifacts/2026-10-11-t992/README.md).

## Linux leased USB microphone PCM (T999)

The Linux accessory build also supports
`blent audio --direction microphone --usb-peer <full-tablet-fingerprint>`.
Microphone uses the same typed original media registration/connection owner as
speakers, with its own invitation, lease, helper and completion proofs. Full
profile, peer and credential validation precede native setup; ADB remains the
default and USB never silently selects it. Android requires explicit original
microphone permission and local Start before preparation (T996–T998).

Authenticated mono PCM uplink preserves negotiated processing, native clock
metadata, buffering and background choice. The native adapter receives bounded
PCM through its existing staging writer; one stalled direction cannot release
its admission or substitute a new original connection. Pending Stop/loss starts
no helper. Actual helper and original route completion remain independently
owned after errors or bounded deadlines, preserving T995 admission behavior.

Permanent private fixtures exercise real mutual TLS, descriptor registration,
management/mux, local PCM and synthetic native helpers in both directions. They
verify exact mono samples/clock, pending Stop, original source loss, failed/stalled
helpers, held actual dependent native leases and delayed remote retirement.
GUI tests cover microphone USB selection, editing, passive persistence and
Start/Stop without changing display. Shared ADB regressions remain.
This establishes implemented routing and automated ownership, not microphone
hardware, acoustic quality or real duplex acceptance. Automated duplex isolation
is T1000; postponed physical T859 and SDK-wide native validation T835 remain.
[Offline evidence and individual native counters](reviews/artifacts/2026-10-11-t999/README.md).

## Host audio transport controls (T979)

The Audio tab provides separate ADB/Direct USB selectors and a full Tablet
fingerprint field for each direction. Linux reports ADB for both directions and
USB for both audio directions; native/physical qualification remains separate. Unsupported
saved targets stay visible and are never rewritten during rendering. Selection,
editing and Apply save passive preferences; they start no connection or media and
do not restart display sharing.

Start validates the entire AudioOptions, including mutually exclusive serial/full
USB peer and profile, before backend construction or admission. It checks the
selected direction's implemented transport availability. Native factories and
capabilities stay behind the platform boundary; controls depend on AudioController
and independent media selection contracts. Camera and audio share the target
editor through focused boundary projections, preserving camera behavior.

Permanent bounded invalid profile/fingerprint tests first reproduced GUI forwarding
of a malformed full target to a permissive backend. The same regression passes.
Interactive tests cover exact speaker USB selection, passive save, manual Start/
Stop, retained processing/buffer/background choices and display independence.
Unavailable-direction and failed-constructor tests reject without retaining a
backend. No physical media result is claimed; speaker acceptance T860 remains
postponed and SDK-wide native-audio validation T835 remains explicit.
[GUI regressions and individual native counters](reviews/artifacts/2026-10-11-t979/README.md).

## Audio admission follows actual retirement (T995)

The controller observes implementation-independent portable completion proofs,
separately from status and backend Result. Each direction tracks a bounded pair:
its original USB connection owner and isolated native helper. The Linux adapters
record proof immediately after native creation, before publishing or returning.
Closed descriptors, requested cancellation, helper exit, timeout and a successful
or failed backend return cannot release Start while actual cleanup remains pending.

Pending cleanup reports Stopping and keeps that direction occupied. It also keeps
its independently retained control-worker completion pending across bounded Drop
or shutdown; microphone, speaker and other controls remain independent. Normal
completion reuses the fixed observation slots. There is no unbounded history,
new native owner or automatic session restart. A backend panic follows the same
retirement barrier before releasing controller admission.

The Linux CLI/pipeline drops the original connection owner before waiting for its
proof, avoiding a self-held barrier. Actual helper and route retirement retain
their existing native owners independently of this observation. Private error,
success, panic, held-lease and sibling-control regressions qualify the ordering;
physical audio acceptance remains T860/T859 and SDK-wide qualification T835.
[Regression and individual native counters](reviews/artifacts/2026-10-11-t995/README.md).

## Native helper argument validation (T993)

The Linux audio helper boundary validates the complete profile and strict
64-character lowercase-hex credential before slicing its label or spawning a
child. Short and multibyte tokens return errors; invalid buffering never creates
a helper. Permanent red/green tests and bounded byte/length cases retain these
checks independently of caller validation. Production callers already generated
valid credentials; wire format and valid native startup remain unchanged.
[Regression and native counters](reviews/artifacts/2026-10-11-t993/README.md).

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
These lifecycle fixes are independent of USB speaker/microphone integration,
which remains planned under T860/T859.

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
   UI/native callback threads. T717 itself supplies no native adapters; the T718/T719 integrations are described below.
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

The grant selects `BLAUD001` (legacy PCM) or `BLAUD002` (PCM plus native counters).
PCM encoding and 10 ms block duration are fixed in both versions.
Integer metadata is big endian; PCM samples are signed **little-endian** 16-bit.
Microphone uses direction 1/mono; speakers use direction 2/stereo, both 48,000 Hz.
The 92-byte handshake must exactly match the authorized session:

| Byte offset | Length | Value |
| --- | ---: | --- |
| 0 | 8 | `BLAUD001` or `BLAUD002`, matching the selected session |
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
| 24 | 2 | Payload bytes: 960/1920 for v1, 984/1944 for v2 |
| 26 | 1 | Authorized direction |
| 27 | 1 | Reserved, must be zero |
| 28 | 0/24 | Native clock prefix, present only in v2 |
| 28/52 | 960/1920 | Exactly one interleaved PCM block |

The v2 prefix contains three big-endian unsigned 64-bit values: native epoch,
frame position normalized to 48 kHz, and native monotonic nanoseconds. All-zero
bytes explicitly mean unavailable. Otherwise epoch/time must be positive and all
three fields must fit a signed 64-bit value for the Android contract. Invalid
counters reject the complete packet without advancing receiver sequence state.

Timestamps strictly increase after the first frame. No subtraction of host and
tablet absolute clocks is used. Replayed/reordered frames, changed direction or
generation, unknown protocol, wrong format, malformed lengths, truncation and
trailing data are rejected. A sequence gap is accepted with a discontinuity and
flushes queued audio. Parsing uses fixed bounded storage; maximum packet size is
1972 bytes (1948 for v1). Authentication is a connection credential check, not encryption or a
per-packet MAC. The wire is intended for the already-authorized ADB connection;
other transport security requires an explicit adapter contract.

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
Intentional speech remains T718; physical duplex, route/USB transitions, measured
acoustic effectiveness and delay remain T720.

## Linux/Android microphone integration (T718)

The implementation now includes microphone Start/Stop in the host **Audio** tab,
`blent audio --direction microphone`, and Android settings. Select **Blent
Microphone** in the desktop application's input chooser after Start. The source
is owned by that session and removed at Stop. Existing default devices are not
changed. Microphone capture starts only after Android permission and authenticated
host readiness; app launch, Apply and reconnect never start audio.

Host settings: speech/raw processing, application buffer 20–200 ms (default 40),
optional tablet serial and background request. Apply saves preferences without
restarting display sharing. Android settings: follow host/speech/raw override,
gain 0–200% (100% default), built-in microphone preference and explicit background
consent. Changing tablet settings ends the active session. Foreground-only capture
stops when hidden; background capture uses a microphone foreground service with
an ownership-bound Stop notification. Permission loss, native errors, Android
silencing, an observed route change or transport loss end the session. Raw mode
requires advertised native unprocessed support. Speech requests Android AEC and
reports availability, without claiming effective acoustic echo removal.

Linux uses an isolated `blent-audio` PipeWire adapter, built with
`--features blent/native-audio`. Source builds need PipeWire/SPA development
headers and libclang (`libpipewire-0.3-dev`, `libspa-0.2-dev`, `libclang-dev` on
Debian-family systems). Packaged builds include the helper. Audio needs the host's
PipeWire client library, SPA modules and running user server; the AppImage uses
that native runtime to match its modules. Missing audio dependencies fail Start
without preventing display sharing. PulseAudio-only, Windows and macOS audio
backends remain unsupported.

The DUMP-protected Android receiver admits an invitation over the authorized ADB
connection. An authenticated 76-byte `BLAUREQ2` bootstrap requests the clocked
protocol; the host still accepts legacy `BLAUREQ1`. The bootstrap carries the invitation
credential (64 hex bytes), capability bits (speech/raw/background), AEC-enabled
flag, direction and effective processing. Human consent has a separate 90-second
budget; it does not consume native startup's five seconds. The host echoes the
invitation credential followed by the 92-byte grant after PipeWire readiness.
Android gives that complete nonce/grant one five-second deadline; partial reads
cannot renew it ([T724 regression evidence](reviews/artifacts/2026-09-29-audio-grant-deadline/README.md)).
Native microphone input preserves sequence-gap discontinuities; the callback uses
only its bounded queue, expires old data and emits silence on underflow. It never
waits on the transport.

Native device-counter integration is implemented for the clocked protocol. Legacy
peers carry no native counters and remain unmeasured. Deploy matching host/Android
builds for v2; an older host does not recognize the new request. Physical duplex
drift/latency and route acceptance remain T720.

## Linux/Android speaker integration (T719)

Start **Speakers** in the host Audio tab or run `blent audio --direction speakers`.
Select **Blent Speakers** in the desktop application's output selector. The owned
PipeWire `Audio/Sink` appears only for this session. Blent leaves existing defaults
and application routes unchanged; it does not mirror the current system mix.
Microphone and speakers have separate controls, saved settings, authentication,
transport, queues and failure ownership. Apply never starts either direction.

Android provides processing override, volume 0–100% for this track, built-in speaker
preference and explicit background consent. It needs no microphone permission for
playback. Speech requests communication playback; Raw requests media playback.
Neither establishes effective acoustic echo cancellation. Actual native route,
buffer frames, played frames and underruns are reported separately from the
application queue setting. An observed route change stops playback and requires
explicit Start; a dead AudioTrack is recreated within its existing authorization,
discarding the interrupted block. Native buffer minima can exceed the requested
application buffering.

Android's speaker controls include **Request low-latency speaker mode**, off by
default. Android may decline the request; session diagnostics report the granted
standard, low latency or power saving mode. Low latency may consume more power.
The native buffer request is unchanged. Muted tablet tests showed about 67 ms less
counter-estimated queued audio, not measured acoustic latency; other routes may
behave differently.

Audio focus denial prevents playback. Transient loss, including duck requests,
pauses and flushes sound; focus regain can resume only the same active session.
While paused, rendering waits for focus events without polling native timestamps;
Stop and transport completion cancel that wait. Permanent focus loss stops it.
Foreground-only playback stops when the app hides.
Consented background playback owns its mediaPlayback foreground service and Stop
notification independently of microphone, camera and display services. Changing
tablet settings stops that direction. App launch or reconnection never resumes it.

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
