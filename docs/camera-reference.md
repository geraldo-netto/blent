# Camera transport and resource ownership

For device setup and controls, read [camera sharing](cameras.md). This reference
specifies the stream format, freshness policy, native cleanup and evidence limits.
An owner retains responsibility for its exact devices/workers until their actual
completion; a Stop request or timeout is not that completion.

## Pipeline and wire format

Linux uses one owned native writer per virtual camera and an FFmpeg decoder for
the selected camera. Each writer configures and writes the original locked file;
it never reopens the device path. Unknown native completion keeps that ownership
held. Closing the containing host process cannot leave an external output writer
running. Android uses Camera2 and MediaCodec.
The camera stream is independent of the display and pen connections. ADB
uses a temporary reverse mapping for its authenticated local TCP connection.
ADB start invitations require an explicit front/rear lens. The probe sends
`com.blent.CAMERA_STOP` with `expected_token` and `expected_port` from its original
endpoint. Android atomically withdraws that endpoint only if both still match;
stale stops leave a newer endpoint intact. The processed acknowledgement is not
native cleanup evidence: original camera/codec owners retain retirement before
replacement. Lensless wire invitations are rejected. Internal local-lens offers
still allow explicit selection on the tablet.
Direct USB uses the brokered accessory channel and a local TCP relay without ADB.
It remains labelled experimental; selected-device physical evidence is described in
[integration status](#direct-usb-camera-integration-status). Both carry the same
authenticated stream. Its `BLCAM002` header
contains the session credential, lens identity and rotation, followed by bounded
big-endian length-prefixed H.264 packets. Packets are capped at 2 MiB; connection,
read/write and encoder-progress deadlines bound stalled sessions. Each new lens
connection gets a new decoder; no encoded history crosses the switch.
Both applications must be updated together. After each complete packet reaches
the host decoder input, the host sends an eight-byte big-endian acceptance counter
starting at one per connection. Android permits only one packet in flight and
rejects unexpected feedback; acceptance is not decoding or presentation.
`BlentCamera` logs every 30 accepted packets: sender-side feedback duration and
extra encoder-queue age relative to the first encoded timestamp. Initial capture
and encoder delay are excluded. No host/tablet clock subtraction is performed;
the sensor timestamp source is logged, but does not establish that MediaCodec
has preserved that clock without compensation.
FFmpeg's decoder pixel budget includes alignment padding; individual allocations
are capped at 64 MiB. Output frames always use the selected desktop dimensions.

## Freshness and recovery

Freshness defaults to 150 ms (`--freshness-ms`, saved `camera.freshness_ms`,
or the Camera tab slider; range 50–2000 ms). This experimental budget controls
extra encoder-queue age, not initial camera/codec latency. A stale encoded frame
is dropped before transmission. All dependent frames are discarded until a
fresh keyframe arrives; one sync request is sent per gap. Codec configuration
is preserved, and a two-second wait without a fresh keyframe stops the session.
Lower budgets favor freshness but can cause more freezing on slow routes.
The sender retains only the current MediaCodec buffer and 8 KiB packet staging;
there is no additional frame-copy queue. The remaining freshness budget applies
to the **whole packet write plus acceptance feedback**, not separately to each
8 KiB segment or received byte. A deadline failure closes that connection;
partially transmitted packets are never continued on another stream. The host
also bounds decoder-input and feedback writes by the configured budget.

Within the still-authorized foreground/background session, transport failure
allows at most two retries with 250/500 ms backoff. Each retry first retires the
old socket, camera and codec, then starts a fresh decoder/encoder generation.
Stop, permission changes, lifecycle cancellation and non-transport failures do
not trigger retries. Retry exhaustion leaves sharing off with an error.

### Adaptive bitrate

Adaptive camera bitrate defaults on. The requested bitrate is the ceiling;
`--min-bitrate` / `camera.min_bitrate` defaults to 1000 kbit/s (effective floor
is capped by the ceiling). Three pressure samples and a one-second cooldown
reduce the target by 25%; a transport failure reduces the next attempt's target
immediately. Five seconds of healthy feedback permit a 10% increase (at least
64 kbit/s), capped at the ceiling. Neutral feedback interrupts the healthy run.
Use the Camera tab checkbox or `--adaptive-bitrate false` for a fixed target.
Lower targets can reduce quality; hardware encoders need not meet them exactly.

[Native recovery measurements and graph](reviews/2026-09-26-camera-freshness.md)
show the tested boundaries. The 150 ms admission budget does not promise a
150 ms recovery time or camera-to-consumer latency.

## Backend boundary

`common/src/camera.rs` defines the portable settings, state and `CameraBackend`
Start/Stop/status contract. `host/src/camera_control.rs` owns the shared manual
session lifecycle. The GUI depends on that contract and chooses a native adapter
at one factory. Linux output ownership/V4L2, FFmpeg launch and ADB mapping live
inside `host/src/camera/`; Android's authenticated wire contract is independent
of the host OS. Windows and macOS require their own virtual-camera output and
lifecycle adapters plus native validation. An interface alone is not platform
support; the Windows preview disables camera Start.

## Current native consumer acceptance

[September 29 acceptance (T621)](reviews/2026-09-29-camera-acceptance.md) verifies
both named V4L2 outputs through real Chrome capture, black inactive/retired frames,
rapid front/rear Stop/Start and final Android resource release on matching builds.
[Physical USB acceptance (T540)](reviews/2026-09-29-usb-camera-readiness.md) also
passes black output on unplug, camera-off reconnect, explicit restart and independent
desktop recovery. That run recorded capture-helper startup at about 12.4 seconds (T558); it is
not a current startup-time guarantee.

## Historical standalone-helper validation (T539)

The [host integration validation](reviews/2026-09-20-camera-host.md) records the
new Camera tab, background capture, rotation/preview checks and installed builds.
The following observations describe the earlier standalone-helper version.

On 2026-09-20, the connected RugKing Pad 2 Pro (Android 16) delivered both rear
ID `0` and front ID `1` through Chrome 153's real `getUserMedia` API at 1280x720.
Chrome enumerated **Blent Front** and **Blent Rear** as separate devices.
Switching, Off, background/resume, restarting capture and restarting the desktop
sidecar were exercised. The inactive device was black, returning to the app
left sharing off, and stopping the sidecar released both Android cameras while
preserving the display's ADB mappings and running service.

Automated T539 tests cover permissions, profile and packet bounds, consent,
foreground lifetime, switching, late callbacks after cancellation, real H.264
decoding, stalled/oversized input, output cleanup and mapping ownership. See the
[implementation validation](reviews/2026-09-20-camera-implementation.md) for
test/coverage evidence and limits. An actual Google Meet call, physical USB
unplug/replug, other tablets and sustained power/latency were not validated.
No camera images were retained in the evidence. The original
[feasibility assessment](reviews/2026-09-20-tablet-cameras.md) preserves the
initial hardware observations. Windows and macOS webcam output backends are not implemented.

## Host frame reuse

Each decoder connection owns a bounded pool of immutable YUV420 frame leases.
`BLENT_CAMERA_FRAME_SLOTS` defaults to 3, accepts 2..16, and takes effect on the
next connection. One additional fixed frame drains input when all leases are held;
no replacement allocation or wait for a stalled reader occurs. Lower capacities
use less memory but can discard more frames when readers are slow. Default pixel
storage is four frames: 5.53 MB at 720p or 12.44 MB at 1080p, excluding allocator
metadata, decoder buffers and the output workers' two fixed black frames. Held
leases survive Stop/reconnect without being overwritten; partial input is never
published.

[T816 measurements](reviews/2026-10-10-camera-frame-reuse.md) cover the Rust frame
staging stage, not end-to-end camera latency.

## Direct USB camera integration status

Camera connection defaults to ADB for existing profiles. In **Advanced camera
settings**, choose **Direct USB** and paste the full **Tablet** fingerprint from
**USB connection** controls. Selecting USB clears the camera's ADB serial; choosing
ADB clears its USB peer. Apply saves only; Start/Restart applies the camera profile
without restarting display. Invalid or unavailable targets fail before backend
creation. Direct USB remains labelled experimental, with physical acceptance
scoped to the API36 RugKing Pad 2 Pro described below. T932 implements
Android's original-consent/isolated-capture composition for direct USB. T973 exposes
it in the regular app's Settings: a USB request shows its lens and requires local
**Start camera** before admission. **Stop camera** affects only camera sharing.
**Allow camera in background** is local consent and is available only when the
computer requests background capture; new owners start with that choice off.
Android permission remains separate from pairing. After withdrawal, the existing
ADB camera controls return. T933 supplies
Linux host registration and pipeline composition in builds with
`accessory-transport`. An explicit `cameras --usb-peer <full-tablet-fingerprint>`
selects that peer independently of display; credentials are fresh per invocation.
Source loss or revocation ends that camera registration. Start a new host camera
invocation to issue another invitation. Builds without the feature fail explicitly
before acquiring camera devices. Production host GUI transport selection is
implemented under T974. [T858 physical acceptance](reviews/artifacts/2026-10-20-media-acceptance/T858/README.md)
passes on the selected API36 RugKing Pad 2 Pro: debugging-on/off delivery, fresh
local consent after software-only reconnect, separate Stop and original native
worker retirement before debugging restoration. Front 640×360 and rear 320×180
delivery are recorded; the latter requires native 640×360 capture as described
above. Older devices and other native providers remain unqualified. Private
TLS/protocol and real H.264 fixtures alone do not establish physical support on
another device, and these bounded observations make no unrestricted latency or
leak-freedom claim.

Starting a new USB connection while the tablet is already in accessory mode
resets that tablet's USB transport to discard bytes from the previous connection.
The native helper first claims the accessory interface; a busy interface rejects
the request before reset. It retains the original native device identity until
a different device object appears at the same physical port, then establishes
accessory mode and claims the new interface. A reset result or elapsed wait alone
cannot admit the old device. This connection operation can briefly interrupt an
ADB desktop connection. Starting camera sharing on an existing authenticated USB
connection performs no USB reset.

The host's direct USB registration allows two minutes for local media approval by
default. Pairing and camera permission remain separate actions within the request
window. The native client's admission setting uses the shared 1–600000 ms resource
policy; startup and cleanup keep their separate short deadlines. Broker
`--resource-timeout-ms`, tablet USB resource deadlines and the client's selected
admission budget are independent: the first expiry cancels the request. Changing
only the broker's limit does not extend the client's budget. Expiry ends the
invitation; it never proves native cleanup or permits an automatic restart.

[Native regression and per-function evidence](reviews/artifacts/2026-10-11-t933/README.md).
