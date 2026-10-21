# T1238 current-build hot-reconnect Stop acceptance

Selected device: RugKing Pad 2 Pro, API36, serial8002RH1010011900, physical USB
3-2.4.4. The designated release certificate signs the current T1247-fixed APK;
the repository verifier passes before local installation. `binaries.sha256.gz`
pins the installed APK and source host/broker/helper artifacts. `sources.json.gz`
pins the production sources at the observation boundary. Host installed services
and AppImage binaries are not replaced; acceptance uses the separate original
source broker/runtime and its retained saved peer identity.

## Debugging-on comparison

Software-only disconnect/reconnect replaces authenticated connection38 with248.
The owner explicitly starts fresh camera, microphone and speaker requests. Front
640×360 at15FPS delivers12/12 distinct frame hashes; speaker playback is audible.
The microphone supplies144000 mono48kHz samples while speakers are active.

An initial reported speaker Stop has no corresponding observed resource exit.
The exact current USB Stop control remains enabled. A recorded tap on its fresh
enabled bounds retires Android speaker2401, host session196709 and helper197906,
and PipeWire endpoint `blent_speakers_45e1f85c5ee0`, object.serial542. The original
microphone2279 / session196988 / helper197847 / endpoint object.serial539 survives
and supplies another144000 samples. Camera frames continue changing. Recorded
microphone Stop then retires its original resources while camera806 / host193275
continues delivering12 frames with11 distinct hashes. Recorded camera Stop
retires that original Android worker and host session. Android exit-info and
separate host reaping provide original-instance proof; disabled buttons alone
are not used. Original authenticated connection248 and display/power workers
remain. This does not establish the cause of any earlier uninstrumented report.

The first audio launch used the wrong cached-SDK plugin environment. T1249
records correction and separate failed/successful setup evidence; those failed
attempts are not represented as passing physical audio sessions.

## Debugging-off hot reconnect

The owner turns debugging off and reopens Blent, keeping the cable attached.
USB18d1:2d00 exposes only the accessory interface; the ADB interface is absent.
Connection3135 authenticates using saved trust. Exact software disconnect and
fresh connect establish authenticated3211 without a cable action. Fresh local
Start is required for all three media requests; no original consent is reused.

Front camera delivers12 frames /11 distinct hashes. The owner hears the short
speaker tone. Microphone `blent_microphone_ce34c6362509`, object.serial636,
supplies144000 real samples alongside speaker endpoint
`blent_speakers_2648abc8bf1f`, object.serial639. The owner corrects an early Stop
confirmation: the button had not yet been pressed. That earlier snapshot is
not evidence of a Stop failure. After the actual local speaker Stop, original
session204248 and native helper204520 exit, and the original sink disappears.
Microphone session204306/helper204494 and the camera remain; another144000
microphone samples and12/11 distinct camera frames verify sibling continuity.

Separate local microphone Stop retires204306/204494 and its source. Camera
session204200 remains and delivers12/11 distinct frames. Separate camera Stop
then retires204200. Five retained pidfds bind exit observations to the original
host process instances and their recorded start ticks; they are all closed when
the observer settles. Each tool-owned media process is separately reaped. Final
PipeWire snapshots contain neither audio endpoint, while the same authenticated
connection3211 remains. `off-all-retired.json.gz` records the host observation
boundary before USB debugging restoration is requested.

After the owner restores debugging and Blent reopens, Android exit-info records
the original speaker4742 at12:08:04.698, microphone4713 at12:09:23.204 and
camera4501 at12:11:36.427, all on2026-10-21 Europe/Rome (CEST). These are actual
SIGNALED/status9 exit records for the selected application's UID10325, on the
same device boot. All precede the saved host observation boundary at12:11:56.508
and the later original connection4429 exit at12:21:02.894 during restoration.
The camera service additionally records original camera4501's close before
restoration. `native-exit-summary.json` retains the checked order and identities;
compressed native dumps retain the original records. Mode-change cleanup is not
used as evidence for these earlier media deaths.

No camera/audio/connection worker restarts after restoration; the native camera
client list is empty and neither PipeWire endpoint exists. Debugging is back on;
global USB-awake override remains0 and the original screen timeout remains15000ms.
This completes the required current-build T1238 Stop comparison and the remaining
original-retirement prerequisite for T858/T859/T860. Earlier uninstrumented Stop
reports still have no established cause; no retrospective root-cause claim is
made. The earlier checkpoint JSON's pending fields describe its capture time;
the later native summary supplies the previously missing evidence.

## Scope and resource boundaries

Only this selected physical device and the original source runtime participate.
Each media request owns its distinct consent, Android worker, endpoint and host
session; local Stop does not revoke pairing or siblings. Original worker death,
child reaping and original endpoint serials supply separate retirement evidence.
PipeWire numeric IDs are reused across generations and are never identity proof.
Snapshots and bounded native observations do not establish unrestricted leak
freedom; the original OS/backend cleanup contracts remain necessary.

Only frame checksums and microphone statistics are retained. Each microphone
consumer admits144000 samples, closes its bounded pipe and is reaped; recorder
exit-13 is recorded rather than claimed as clean success. Host media exit1 with
`USB media registration unavailable` is likewise recorded, not described as a
clean-success exit. No raw camera images or microphone PCM are retained.

Existing normal-suite regressions and per-method native counters under
T1241/T1242/T1245/T1247/T1248 remain supporting evidence; no new production bug
is established by these acceptance observations. T1244 still records unavailable
native SonarQube measurements. No sustained drift/AEC/acoustic-quality campaign,
other-device support, Windows/macOS acceptance, EVDI qualification, default audio
device change, package download, repository fetch/push or artifact upload occurs.
Earlier profile/background/denial/revocation evidence remains historical and
separate; this run does not claim to repeat every earlier condition.
