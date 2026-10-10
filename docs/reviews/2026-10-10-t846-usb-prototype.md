# T846: USB-first ADB-free transport feasibility

Owner selected USB first on 2026-10-10. Keep optional ADB fallback during
qualification; direct LAN remains later. T846 is not resolved by this document
or by compiling the prototype. Production still uses ADB.

## Architecture and prototype scope

The existing [T602 research](2026-09-26-installation-research.md) identifies ADB's
transport, discovery, launch and shell-control roles. The production design
must replace those through independent connection/identity/channel contracts,
with Linux USB and Android accessory adapters. Existing attachment generations,
bounded media queues and isolated media workers should be reused. Do not move
native USB operations into the UI or weaken DUMP-protected receivers.

The first decision gate is an isolated synthetic accessory probe, with no access
to Blent session tokens, screen contents, camera, microphone or input injection:

- `scripts/benchmarks/aoa/contract.c` validates physical USB target coordinates;
  `usb_probe.c` owns stock libusb discovery, accessory switching and bulk I/O;
  `main.c` owns the bounded workload and worker supervision. No dependencies are
  patched. Device selection uses exact bus/physical port path, never the first
  arbitrary Android device. It discovers the accessory endpoints from the active
  configuration and does not claim or detach the ADB interface.
- Profile/optimized Android variants include a separate `Blent USB Probe`
  launcher/attach activity in private process `:aoa_probe`. Debug/release app
  variants do not include that activity or its native I/O controller. Existing
  DUMP-protected diagnostic activities retain their guards. Accessory permission
  is checked through UsbManager, using its actual accessory inventory rather
  than trusting launch extras.
- `AoaProbeWire` is a pure JVM framing/integrity contract. Headers contain magic,
  session, sequence, payload length and CRC32; payloads are bounded to 64 KiB,
  sessions to 4,096 packets. Host tests cycle 1, 64, 512, 4,096, 16,384 and 65,536
  byte payloads. Echoed bytes must match the generated pattern exactly.
- This checksum/session protocol is **not authenticated pairing** and cannot be
  promoted to production authority. It carries synthetic test bytes only.
- Android allocates one fixed header/payload pair. It rejects invalid sizes,
  stale sequences/sessions, corrupt data and zero-progress reads. Native open,
  read, write and close run off the UI thread. One run is admitted per process;
  Stop closes on a separate thread and terminates that private process after a
  second, so a stuck close cannot strand replacement owners. A 60-second absolute
  watchdog remains active through native close and result persistence.
- Host native operations run in one child, with finite libusb transfer deadlines
  and a 70-second child alarm. Parent waits at most 75 seconds, then kills and
  allows two seconds for retirement. A child-held, per-port ownership lock
  prevents another probe taking over while an unretired child remains. Kernel
  hangs are not claimed recoverable. Only a successfully claimed interface is
  released by cleanup.

The host CSV records synthetic echo roundtrip time, not video/render latency.
Checksumming, USB framing and Android echo work affect these measurements. It
does not yet test fair production channel multiplexing, authenticated pairing,
codec/render delivery or current shared-media workers. No USB-versus-LAN speed
claim follows from compilation or the historical protocol-2 query.

## Offline validation prepared

Cached Android SDK34, stock host libusb runtime and the already installed
Freedesktop SDK25.08 libusb header suffice; no download is needed.

```sh
sh scripts/benchmarks/aoa/build.sh /tmp/blent-t846
python3 -m unittest discover -s scripts/tests -p test_aoa_probe.py
./android/gradlew -p android --offline :app:testDebugUnitTest \
  --tests com.blent.AoaProbeWireTest :app:assembleProfile :app:assembleOptimized
```

The permanent normal suites include six JVM wire tests and seven native tests.
Native contract/wire fixtures run with ASan and UBSan and do not open USB devices.
They cover identical C/JVM golden bytes, all workload payload sizes, malformed
headers, corrupted payloads, partial/truncated/zero-progress reads, stale sessions,
exact packet-budget termination, numeric/port boundaries and bounded invalid-input
fuzz cases. Both Android benchmark variants build offline. Benchmarks retain the
existing coverage exemption; this adds no production transport implementation.
The compile warning in existing VideoTiming code is separately tracked as T847.

Review reproduced premature failure when a re-enumerating node exists before udev
permissions settle or disappears before opening. The unchanged permanent tests
failed before the fix, then passed with the real adapter against fake libusb:
ACCESS/NO_DEVICE retry only during re-enumeration; persistent errors exhaust the
100-attempt budget and report the last open error. Initial selection and other
errors remain strict. Fake sleep makes the bounded-failure tests deterministic;
no real USB node is opened. The host rebuild passes strict compiler warnings;
all 23 host C functions have cyclomatic complexity at most 8. Profile-source-set
wire tests, worker-manifest tests and license-asset tests also pass offline.

[Retained evidence](artifacts/2026-10-10-t846/README.md) includes red/green logs,
Android checks and source/binary/APK hashes. Debug APK inspection excludes the
probe activity; profile and optimized APKs include it. These are build and unit
test results, not native USB acceptance.

## Live test prerequisites and recovery

The prepared separate APK is
`android/app/build/outputs/apk/profile/app-profile.apk`, package
`io.github.geraldo_netto.blent.profile`. Installing it must not replace the normal
production package. Do not grant camera/microphone permissions for this test.

Before a live mode switch, the maintainer must be ready to operate the tablet:

1. Install the separate profile APK while existing ADB works, then stop current
   Blent streaming and open **Blent USB Probe**.
2. Disable USB debugging through Android settings. Host records the service state
   and exact physical USB path before testing; on October 10 the tablet was at
   bus 3, port path `2.4.4`, speed 480 Mbit/s. Rediscover rather than assuming that
   location remains valid after cable/hub changes.
3. Run `/tmp/blent-t846/aoa-probe BUS PORT.PATH NEW_RESULT.csv`. This sends AOA
   identification/start requests and temporarily changes that tablet's USB mode.
   Approve Android's accessory access/open-app prompt; tap **Start USB test** if
   needed. Host waits ten seconds before its bounded transfer attempt. A timeout
   is a failed attempt, not a reason to bypass permission.
4. Record actual accessory-only USB descriptors and matching complete host/app
   results. Exercise Stop, unplug/replug, denial and a second fresh process before
   calling native lifecycle qualification complete. Android keeps a private
   `aoa-probe-result.txt`; host CSV requires no ADB during the test.
5. Finish/stop the probe, release handles, then physically unplug/replug. Closing
   libusb is not a portable command to restore the original USB composition.
   Re-enable USB debugging and approve its host prompt only when returning to the
   existing ADB deployment, then restore the previous host service state. If the
   firmware retains accessory mode, use Android USB settings or a manual reboot;
   do not reset unrelated USB devices or assume recovery succeeded.

These actions require tablet interaction and briefly interrupt current Blent/ADB.
No mode switch, installation, debugging-setting change or host service restart is
part of the build-only checks above. Native acceptance is still pending.

## Native synthetic trial (2026-10-10)

With the maintainer ready, installed the separate profile APK, recorded the active
host service, paused it and launched the probe. The maintainer disabled USB
debugging. The selected physical port changed to `18d1:2d00`, with one `ff/ff/00`
accessory interface and no ADB interface.

Attempts 1 and 4 echoed payloads 1, 64 and 512 bytes, then failed. Added transfer
diagnostics identified a successful zero-length USB bulk IN packet (`status=0`,
`count=0`, 24 header bytes remaining). Our byte-stream probe incorrectly rejected
this valid transport event. Attempt 2, without physical detach after failure,
rejected a mismatched header, consistent with residual endpoint bytes but not
sufficient to establish their origin. Attempt 3 failed during re-enumeration with
an unrecorded open error; improved diagnostics now retain that status. Its exact
cause and any additional retry policy remain unqualified.

The permanent fake-libusb regression fails before the zero-length packet fix and
passes afterward under ASan/UBSan. Reads now tolerate up to eight consecutive
zero-length packets within a five-second deadline for the whole transfer; writes
must still progress. Positive partial transfers reset the empty-packet count.
Additional cases reject endless empty reads, zero-progress writes, USB errors,
negative/oversized counts, invalid lengths and an expired transfer deadline.
All eight native tests pass. All 27 host C functions have cyclomatic complexity
at most 7. No stock libusb or Android dependency code was changed.

After a fresh physical connection, attempt 5 passed all **4,096 patterned frames**:
**59,061,099 payload bytes each direction**, using the same Android APK with
USB debugging disabled. Host verified every echoed header and payload. Raw CSV,
logs, descriptor snapshots, red/green regressions and native source/executable
hashes are retained in [live evidence](artifacts/2026-10-10-t846/live/).

| Payload bytes | Echo roundtrip p50 (ms) | p95 (ms) |
| --- | --- | --- |
| 1 | 0.811 | 1.592 |
| 64 | 0.563 | 1.415 |
| 512 | 0.584 | 1.427 |
| 4,096 | 0.759 | 1.527 |
| 16,384 | 1.540 | 2.467 |
| 65,536 | 5.229 | 6.570 |

These are one successful synthetic run, including Android echo and USB overhead;
not video latency, steady-state throughput or a matched ADB/LAN comparison.
Android's saved result independently confirms `PASS: 59061099 payload bytes
echoed; 4096 checked frames`, with source identity matching the tested profile APK.
The maintainer stopped the probe, unplugged/replugged and re-enabled debugging.
ADB returned, the previously active host service was restarted, normal Blent
launched and repeated tablet render-callback acknowledgements confirmed recovery.
The production APK was not replaced; the separate profile test APK remains
installed. Recovery evidence is retained alongside the trial data. Permission denial, mid-transfer
unplug/stall/death and independent clean reconnect qualification remain open.
The prototype success does not establish production authentication, multiplexing,
media integration, performance, charging or other-platform support.

## Work after the native gate

Specify portable peer identity, authenticated pairing/session, attachment epoch,
channel capabilities, bounded envelopes and shutdown contracts. Keep OS descriptors
and lifecycle adapters outside them. Prototype fair per-channel flow control before
sharing one USB stream across display, touch, camera and audio. App-native control
must replace shell-delivered secrets and invitations without removing user consent.
Integrate discovered accessory connections with current host session ownership and
Android media workers, then validate ordinary rendering, recovery, memory, CPU and
charging. Preserve explicit ADB fallback and visibly unsupported native backends;
Windows USB qualification remains distinct and macOS remains out of scope.
