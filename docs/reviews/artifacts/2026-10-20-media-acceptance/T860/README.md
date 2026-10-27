# T860 native speaker acceptance checkpoint

Selected-tablet speaker checks verified audible playback and original endpoint/worker retirement while siblings continued. These observations qualify the stated device and scenarios, not acoustic quality, sustained drift or other providers.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T860 was in progress at this checkpoint pending T1238's final Stop comparison.
The October21 completion below supplies that missing evidence. Tests here used
the API36 RugKing Pad 2 Pro, signed T1235 Android release,
source host/broker and production native adapters. The installed host AppImage
has not been replaced. No protocol fixture substitutes for physical sound.

With debugging off (18d1:2d00, no ADB interface), connection112 created a real
48kHz stereo PipeWire sink. Owner heard the isolated short tone. Android local
speaker Stop ended its original host session and removed sink92 while microphone
source102 kept delivering real PCM; separate microphone Stop removed that source.

Connection6370 again delivered a confirmed speaker tone alongside camera and
microphone. Exact original-peer revocation removed its host trust/link, reaped
all three original host sessions and removed both audio endpoints. Software-only
T1237 transport reset established8535 without unplugging. Fresh Android Start
was required for each new offer. A new sink with object.serial4186 delivered a
second owner-confirmed tone while microphone and rear180 camera worked.
Numeric PipeWire node IDs can be reused; original endpoint names and object
serials, rather than reused node numbers, distinguish these lifetimes.

The later uninstrumented report of Stop on8535 did not correspond to observed
host cleanup; T1238 tracks it. Changing USB debugging later retired those native
media workers. A subsequent controlled production inspection on9641 tapped the
exact enabled USB Stop speakers button. Original speaker18020 exited and its
sink disappeared, while original microphone17987, camera17788 and desktop stayed
live. Original microphone/camera Stops then independently retired their workers
and endpoints; native process exit records and final process observations remain.
Session exit1 on loss/Stop is recorded, not described as clean success.

Retained prerequisite regressions cover original authority, denial/cancellation,
independent sibling retention, actual consumer retirement and late/unknown
cleanup. T1212 records private Linux PipeWire/native per-function coverage;
T1226–T1237 record prerequisite fixes, permanent red/green regressions and
individual native counters. This trial establishes audible playback on one
selected tablet. It does not qualify sustained clock drift, acoustic quality,
echo cancellation, other hardware or excluded backend/distribution suites.

## Completion — 2026-10-21

The [current signed-build T1238 comparison](../../2026-10-21-media-acceptance/T1238/README.md)
passes after debugging-off software-only reconnect3135 to3211 and fresh local
Start for each media request. The owner hears the bounded speaker tone. Original
speaker sink `blent_speakers_2648abc8bf1f`, object.serial639, is removed after the
actual local Stop; original host204248 and helper204520 exit and are separately
reaped. Retained pidfds bind observations to those process instances and their
recorded start ticks, without relying on reusable PIDs or node numbers alone.

Microphone source object.serial636 survives, supplying another144000 real samples;
camera delivers another12 changing frames. Original pairing3211 remains
authenticated. Separate microphone/camera Stops then retire their own resources.
After debugging restoration, Android exit-info identifies original speaker4742's
actual death at12:08:04.698 CEST, before the saved cleanup observation and later
connection4429 death at12:21:02.894. No speaker worker or original sink remains.
The debugging-on exact-control comparison independently retires speaker2401 and
its original host/helper/sink while preserving microphone and camera.

This resolves T860's remaining original-retirement prerequisite. Earlier exact-peer
revocation, local denial/cancellation, coexistence, background and native-clock
results remain scoped historical or automated evidence, rather than claims that
this last trial repeats each condition. The normal-suite invalid/boundary/lifecycle
regressions and individual native counters remain permanent. Audible playback on
one selected API36 device does not qualify sustained drift, acoustic quality, AEC,
other native providers or excluded/unsupported backends. Earlier uninstrumented
Stop reports have no established cause; the corrected early confirmation in the
latest trial is not a Stop failure.

</details>
