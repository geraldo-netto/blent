# Tablet cameras in Blent host

The host **Cameras** tab controls camera sharing: Front/Rear, resolution, frame
rate, bitrate, clockwise rotation (0°, 90°, 180° or 270°), mirroring and whether sharing continues while the tablet app is
hidden or its screen is locked. Lens and resolution stay visible; the remaining
profile controls are under **Advanced camera settings**. Android shows status,
its required camera permission prompt, and explicit **Start camera** / **Stop
camera** actions; camera profile configuration stays on the computer.

Choose a lens and press **Start camera** on the computer. Open Blent on the
tablet, then press its separate **Start camera** and grant camera permission
if requested. Pairing alone does not start capture. In Google Meet or another application, select
**Blent Front** or **Blent Rear**, matching the host selection. Both ordinary
Linux webcams exist while sharing runs, but only the selected lens is live;
the inactive endpoint is black. A browser's webcam picker does not switch lenses.

The small live preview shows the selected webcam’s exported image, including
rotation and mirroring. It uses a thumbnail at up to five updates per second; it
does not open a second camera. Stopped or stale previews clear rather than
retaining the last camera image. Select **180°** when the tablet is upside down,
then **Restart camera** to apply the change.

The camera helper is embedded in the host GUI through a reusable host library.
No separate helper command or display daemon is needed. Keep the host window
open while sharing; **Stop camera** or closing that window requests capture
cleanup and black output. A replacement waits for the original output workers
and FFmpeg decoder and, for direct USB, the original registration to retire.
Unconfirmed cleanup keeps replacement pending; Stop also cancels that pending
start. Legacy ADB
reverse-map cleanup still has an ownership race with concurrent external changes
(T1090). Camera operations never attach EVDI or restart display sharing.

**Apply** saves camera preferences without starting capture. **Start camera**
uses the current controls; **Restart camera** applies changed controls to an
active camera. Opening the host never restores capture automatically. After
an error, correct its cause and press Start/Restart to retry. The tablet can stop
capture or restart the existing computer request while that session remains
available. Its Start action is disabled without a request. Tablet Stop ends
capture without stopping display/input or deleting the computer’s camera outputs;
use Stop on the computer to retire the entire camera session.

**Continue while hidden or locked** defaults off. When enabled, Android starts a
camera foreground service with a persistent notification and holds a CPU wake
lock until capture ends. Start with Blent visible on the tablet; Android's
[camera service restrictions](https://developer.android.com/develop/background-work/services/fgs/service-types#camera)
require foreground permission to initiate this service. Once running, the
Activity may hide or the screen may lock. Lock/sleep actions can require a
manual Android unlock before later foreground-only sharing; the host does not
bypass the tablet’s lock. Foreground-only mode stops when the
Activity hides; reopening it can resume an outstanding host request while its
session remains available. Background mode costs additional tablet power.

Camera sharing carries video only. Optional [tablet microphone and speaker
sessions](audio.md) are implemented separately on Linux/Android, with independent
Start/Stop controls and permissions. Microphone physical acceptance and
simultaneous duplex/AEC validation remain tracked in `TODO.md`; camera acceptance
does not establish those results. NFC remains planned separately.

## Prepare the devices

Install your distribution's `v4l2loopback` module package, ADB and FFmpeg. Module
installation/loading may require administrator access and Secure Boot signing.
Load two dedicated devices before starting the command:

```sh
sudo modprobe v4l2loopback devices=2 video_nr=20,21 \
  card_label="Blent Front,Blent Rear" exclusive_caps=1,1
```

If the module is already loaded with different options, close its consumers
first, unload it with `sudo modprobe -r v4l2loopback`, then run the load command.
Do not unload devices another application is using. The command checks labels
and takes ownership locks; it does not relabel or replace unrelated webcams.
Your user must have read/write access to these nodes, normally through the
desktop's device-access policy or the `video` group.

Use the updated Android APK and host GUI from the same checkout, preserving the
[release signing identity](release-signing.md). Connect the tablet through
authorized ADB. Leave **Tablet serial** empty with one connected tablet; enter
its ADB serial when multiple tablets are connected. Refresh/reopen a browser's
camera picker if it cached devices before sharing started. A full Stop may end
a browser’s capture track; select the webcam again after a later Start.

For an unreleased checkout, build with `cargo build -p blent-gui` and run
`./target/debug/blent-gui`. Building the checkout does not update an older
installed AppImage automatically.

## Diagnostic command and profile limits

The CLI remains available for headless diagnostics, with the same pipeline:

```sh
blent cameras --lens rear --background --serial TABLET_SERIAL \
  --width 1280 --height 720 --fps 30 --bitrate 3000 --rotation 180 \
  --front-device /dev/video20 --rear-device /dev/video21
```

The command explicitly starts the selected lens (default Front); omit
`--background` for foreground-only capture. Ctrl-C/SIGTERM requests stop and
awaits the original pipeline cleanup. A Ctrl-C registration error also requests
stop and awaits cleanup before returning its diagnostic. Do not run
it alongside GUI camera sharing: both enforce exclusive output ownership.

Default capture is 1280x720, 30 FPS and a 3000 kbit/s H.264 target. Optional
`--mirror` mirrors exported video. Width accepts even values 160–1920; height
accepts even values 120–1080; FPS accepts 5–30 and bitrate accepts 256–20000
kbit/s. These are request limits, not guaranteed modes: Android rejects profiles
its camera does not advertise. A bitrate target is not a measured traffic rate.
Larger profiles cost USB bandwidth, decoding/encoding work and tablet power.

The resolution chooser also offers **320×180 (180p)** and **640×360 (360p)**
for low-detail calls. These preserve the same explicit Start and Android consent.
The 180p output deliberately uses native 640×360 capture and host downscaling;
both presets therefore require the camera to advertise 640×360. Other profiles
use their selected dimensions for native capture. Unsupported native sizes report
an error. The default remains 720p. Lower FPS and bitrate can further reduce the
tablet-to-computer stream. The calling application's own resolution, bitrate and network behavior
determine internet usage; the Blent camera bitrate controls its USB stream.

Linux uses one owned native writer per virtual camera and an FFmpeg decoder for
the selected camera. Each writer configures and writes the original locked file;
it never reopens the device path. Unknown native completion keeps that ownership
held. Closing the containing host process cannot leave an external output writer
running. Android uses Camera2 and MediaCodec.
The camera stream is independent of the display and pen connections. Legacy ADB
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
desktop recovery. Capture-helper startup still took about 12.4 seconds (T558).

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
worker retirement before debugging restoration. Front640×360 and rear320×180
delivery are recorded; the latter requires native640×360 capture as described
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
window. The native client's admission setting uses the shared1–600000ms resource
policy; startup and cleanup keep their separate short deadlines. Broker
`--resource-timeout-ms`, tablet USB resource deadlines and the client's selected
admission budget are independent: the first expiry cancels the request. Changing
only the broker's limit does not extend the client's budget. Expiry ends the
invitation; it never proves native cleanup or permits an automatic restart.

[Native regression and per-function evidence](reviews/artifacts/2026-10-11-t933/README.md).
