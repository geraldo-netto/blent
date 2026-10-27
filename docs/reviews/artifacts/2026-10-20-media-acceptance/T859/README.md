# T859 native microphone/duplex acceptance checkpoint

Selected-tablet microphone/duplex checks verified real PCM, independent Stop and later original Android worker exit. Bounded sample statistics do not establish intelligibility, acoustic echo cancellation or sustained clock stability.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T859 was in progress at this checkpoint pending T1238's final Stop comparison.
The October21 completion below supplies that missing evidence. Tests here used
production source Linux adapters and signed T1235 Android
release on the API36 RugKing Pad 2 Pro. The installed host AppImage is unchanged.
Only bounded PCM statistics are retained; raw microphone samples were discarded.

With debugging off (18d1:2d00, no ADB interface), connection112 admitted explicit
Android speech-mode microphone consent at60ms buffering. Real PipeWire source102
provided144000 48kHz mono samples while speakers ran:35559 nonzero, peak36,
RMS4.09095. After separate speaker Stop removed sink92, the original microphone
continued:144000 samples,7209 nonzero, peak16, RMS1.01229. Microphone Stop ended
its host session and removed its source. Raw processing was explicitly rejected
as unsupported on this tablet while the speaker sink survived.

Connection6370 supplied another144000 real samples with camera and speakers.
Exact-peer revocation reaped all three host sessions and removed their endpoints.
After software-only T1237 reset/reconnect8535 and fresh explicit local consent,
a new source (object.serial4183) supplied144000 samples with camera and speakers.
`ready-microphone.json` and `reconnected-microphone.json` retain exact statistics.
Their bounded consumers close stdout after the sample capacity; recorder exit-13
is recorded rather than represented as clean success.

The later uninstrumented Stop report on8535 did not match host cleanup; T1238
retains that discrepancy. Transport mode change subsequently retired its native
workers. On controlled fresh9641, exact USB speaker Stop retired speaker18020
without disturbing microphone17987/camera17788. Exact microphone Stop then
retired17987 and its source while camera and desktop remained. Camera Stop
subsequently retired17788; native exit records/current process observations are
retained. Host session exit1 is not presented as clean-success termination.

T1212 supplies private Linux transport/native per-function coverage. T1226–T1237
retain prerequisite regressions/counters for actual original native retirement,
Binder validation, pending cancellation, projection, UI and awake ownership.
Existing joined duplex tests retain denial, permission loss, original/sibling
ownership, late callbacks and delayed actual cleanup coverage. One observed
-691ppm native correction is not a sustained drift qualification. Speech-mode
admission and real PCM do not establish intelligibility, acoustic AEC performance,
other providers/devices or unsupported/excluded backend qualification.

## Completion — 2026-10-21

The [current signed-build T1238 comparison](../../2026-10-21-media-acceptance/T1238/README.md)
passes after debugging-off software-only reconnect3135 to3211, with fresh local
Start for each media request and no ADB interface. Speech-mode microphone at60ms
buffering supplies144000 real mono48kHz samples while the owner-confirmed speaker
tone and camera are active:9904 nonzero, peak12, RMS1.06091. After separate
speaker Stop, the original microphone source
`blent_microphone_ce34c6362509`, object.serial636, supplies another144000 samples:
54979 nonzero, peak26, RMS3.75326. Only statistics remain; raw PCM is discarded.

Local microphone Stop reaps original host204306 and helper204494 and removes its
original source. Camera204200 still delivers12 changing frames /11 distinct
hashes; pairing3211 remains authenticated. Original host identities are retained
through pidfds and recorded start ticks until actual exit. After debugging
restoration, Android exit-info records original microphone4713's death at
12:09:23.204 CEST, before the saved cleanup boundary and connection4429's later
12:21:02.894 exit. No microphone worker or original source remains. Independent
debugging-on exact Stop similarly retires microphone2279 and its host/helper/source
while camera keeps delivering frames.

This resolves T859's remaining original-retirement prerequisite. Earlier native
raw-mode rejection, exact-peer revocation, requested/local background, denial and
clock observations remain scoped historical or automated evidence; this final
trial does not repeat every condition. The permanent normal-suite ownership,
invalid-input, permission, fault and interleaving regressions and individual native
counters remain. Bounded PCM delivery and sibling continuity on this selected
API36 device do not establish sustained drift, intelligibility, acoustic AEC,
other providers/devices or excluded/unsupported backend qualification. Native
recorder exit-13 and host session exit1 are recorded rather than claimed as clean
success. Earlier uninstrumented Stop reports have no established cause.

</details>
