# T858 physical acceptance checkpoint — 2026-10-20

The October 21 follow-up completed selected-tablet camera retirement after debugging-off software reconnect. Earlier profile and revocation observations remain scoped; the uninstrumented prior Stop report has no established cause.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

This October20 checkpoint was incomplete. The October21 completion below supplies
the missing current-build original-retirement evidence; T858 is complete for this
selected device and retains the explicit limits described here.

Production source host/broker/helper and signed release Android APK were used,
not protocol fixtures. Android build is the T1235 installed APK; host binaries
include T1234. The globally installed host AppImage has not been replaced.
Only the selected RugKing Pad 2 Pro (serial8002RH1010011900, USB3-2.4.4)
was touched. USB debugging was off; accessory18d1:2d00 advertised no ADB
interface. Owner confirmed Blent visible and explicitly started both offers.

- Original authenticated connection20784 used Computer c6c673… / Tablet71553f….
- Front640×360 at15FPS/512kbit target:12 exported frames,9 distinct hashes.
- Owner tapped Android Stop: original host camera process998051 exited1 with
  `USB media registration unavailable`; authenticated USB link remained.
  No clean-success exit is claimed. Decoder reported broken-pipe on teardown.
- Fresh Rear320×180 at15FPS/256kbit target: native640×360 capture projected
  to180p. See actual frame hashes; no images or raw audio are retained.
- Exact original-peer Revoke used production Control command4, guarded against
  connection20784/epoch1/full tablet identity and the physical USB selection.
  Camera session exited1, link list became empty, durable host trust disappeared.
- T1237 permanently fixes queued discovery bytes across a logical USB reconnect.
  With debugging off, authenticated6370 delivered12 front360 frames/11 distinct
  hashes. Exact-peer revocation retired its three original host media sessions
  and audio endpoints. Software-only reset/fresh connection8535 authenticated
  without a cable action; fresh explicit Android consent delivered12 rear180
  frames/11 distinct hashes. [Transport proof](../../2026-10-21-t1237/README.md).
- Four permanent native camera original-owner retirement tests pass unchanged;
  log retained. These tests complement physical evidence, rather than establish
  remote Android worker death while debugging is off.

Controlled debugging-on USB Stop inspection on fresh connection9641 retired
original camera17788 after its exact enabled Stop button was tapped. Speakers
18020 and microphone17987 had independently retired first, preserving camera
and desktop. Native process exit records and current process observations are
retained; host exit1 is not claimed as clean success.

At this checkpoint T1238 remained: the owner previously reported all controls disabled after Stop
on debugging-off hot connection8535, while camera frames still changed and host
resources remained. No UI action was instrumented in that episode. Subsequent
exact Stop controls pass; fresh hot-reconnect10450 offers expired without local
approval, so that final comparison remains unverified. Transport mode change
retired the8535 media workers; it does not prove local Stop worked. Host exit
and an empty link snapshot alone are not proof of Android native retirement.
Other-device/backend support and acoustic/microphone acceptance are separate.
Tablet global USB-awake override was already restored to0 before these trials;
its original15000ms screen timeout remains unchanged. The T1235 application
window policy holds the screen only while foreground media/display is active.

## Completion — 2026-10-21

The [current signed-build T1238 comparison](../../2026-10-21-media-acceptance/T1238/README.md)
verifies fresh local front-camera Start after debugging-off software reconnect
3135 to3211, with no cable action or ADB interface. The original camera continues
delivering changing640×360 frames after separate speaker and microphone Stops.
Each bounded12-frame observation has11 distinct hashes. Camera Stop then reaps
original host session204200 while pairing3211 stays authenticated.

After debugging restoration, Android records original camera4501's actual exit
at12:11:36.427 CEST and the camera service records its earlier device close.
These precede both the saved host cleanup observation and later connection4429
exit at12:21:02.894 during restoration. No camera worker or active camera client
remains. The original native lifetime is not inferred from host exit or mode
change alone. Debugging-on exact controls independently retire camera806 with
its original host193275 after preserving the audio/display siblings.

This closes the remaining T1238 prerequisite for T858. Earlier front/rear,
profile, lifecycle and exact-peer revocation observations remain scoped
historical evidence; the October21 trial does not claim to repeat every profile.
The normal-suite ownership/invalid/stale/interleaving regressions and scoped
per-function counters remain permanent. Older providers/devices and other OS
backends remain unqualified or unsupported as explicitly documented. No raw
camera images are retained, and no cause is inferred for the earlier
uninstrumented Stop episode.

</details>
