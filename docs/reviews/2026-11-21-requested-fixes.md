# Requested findings — 2026-11-21

Baseline: `58f58a32`. All 25 findings explicitly listed by the owner now have local verified fixes; T1622's original deletion claim was corrected by native evidence before fixing extraction ownership and cleanup. Physical-device acceptance remains separately tracked; the additional T1629 capture fixture failure is now explained and fixed. Existing unrelated TODO additions remain unstaged. Fix verification used private fixtures. The subsequent authorized local reinstall/reload is recorded under T1642/T1643 below. No push or publication.

## T1615 — capture pause acknowledgements

Confirmed both missing transitions with permanent failing-first regressions: a new pause after capture is already quiescent, and a pause arriving while original native cleanup is held. Capture now acknowledges the current request after cleanup and while it remains parked. Three consecutive pause/resume transitions pass without a display/settings event. Held native cleanup still prevents acknowledgement; the existing cancellation and sibling-isolation assertions are retained.

All six quiescence cases pass. Five maintained production methods measure 100% executable-line coverage; 22 functions in the changed source/test scope have complexity <=7. [Evidence](artifacts/2026-11-21-requested-fixes/t1615-verification.json). No resource owner or queue is added: the capture worker retains the authoritative quiescent state until new capture admission, and exact helper/encoder settlement remains the prerequisite. Physical host-lock/display-sleep acceptance was not run against the user's session.

## T1629 — expired packets retained by the test fixture

The original run exposed a fixture queue-clearing bug. With a 400 ms delayed consumer, the same native test reproducibly received an expired prior-generation delta after resume. `try_recv` first reported Lagged; the fixture mistook that notification for an empty queue. A permanent bounded overflow regression failed before correction. The shared fixture helper now drains through Lagged to Empty/Closed after the original producer is quiescent. Production behavior and every generation/IDR/cleanup assertion remain unchanged.

All 26 activity cases and all 247 executed capture cases pass; two existing opt-in experiments remain ignored. All five changed test functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1629-verification.json). The delayed native consumer remains a permanent interleaving regression.

## T1489 — VAAPI quality option

Confirmed against installed FFmpeg option tables and [FFmpeg 6.1.1 rate-control source](https://github.com/FFmpeg/FFmpeg/blob/n6.1.1/libavcodec/vaapi_encode.c#L1807-L1816). VP9/AV1 VAAPI now receive `-global_quality`; H.264/HEVC retain `-qp`. The configured value is passed unchanged and the documentation distinguishes codec quantizer scales. The permanent regression failed before the fix and now covers all five VAAPI profiles at minimum/default-example/maximum values. All 14 existing encoding unit cases pass. Both maintained production methods have 100% line coverage; all 26 measured source/test functions are <=9 complexity. [Evidence](artifacts/2026-11-21-requested-fixes/t1489-verification.json). No hardware encode was run.

## T1538 — explicit ADB refusal settlement

Confirmed with failing-first socket and DNS reply regressions. A completed successful `adb connect` command with an exact bounded socket/DNS failure reply now proves noncreation; a private one-slot attempt test confirms later reconnection and shutdown without cleanup polling. All 39 Wi-Fi cases pass, four maintained production methods have 100% line coverage and all 40 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1538-verification.json).

Upstream ADB distinguishes these failures before transport registration from generic connection/authentication failures after registration. The latter, timeouts, nonzero command exits and malformed replies remain uncertain and can still suspend recovery until their original outcome settles. Documentation now states this limitation. No live server connection or Windows native test was performed. The recognized connect reply strings do not apply to reverse-map creation.

## T1595 — broker accept failures

A private native descriptor-exhaustion regression failed before the fix because `accept4` EMFILE terminated the broker. Accept failures now delay only further accepts by 100 ms; existing clients and links continue service. Permanent tests confirm service during exhaustion, admission after capacity returns, and service after actual SO_PEERCRED rejection against a deliberately different expected UID. All 256 broker cases pass under instrumentation. Three maintained methods exceed 91% line coverage; all 21 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1595-verification.json). Existing owners and exact shutdown settlement remain intact.

## T1594 — automatic USB selection and retry

Three failing-first regressions confirmed unrelated-device selection, fixed retries and repeated native discovery. Automatic attempts now require Android accessory identity; normal-mode tablets require explicit Connect. Startup failures back off from 2 to 60 seconds, authentication resets the history, and native discovery without Hello suppresses retries until confirmed replug or manual Connect. Two additional tests cover authentication reset and detach during original cleanup. Existing automatic-server fixtures now identify accessory candidates explicitly; their original service/ownership assertions remain.

All 256 broker cases pass; all 11 maintained production methods reach at least 95% line coverage, and all 66 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1594-verification.json). Retry records remain controller-owned and bounded by admitted device keys; exact original native retirement still precedes reuse. No connected USB hardware was probed.

## T1540 — display route refresh

The failing-first regression confirmed that a read-only refresh error discards the original worker. Identity/listing failures during refresh now report the error while retaining live mappings and their original owner. Initial admission errors, failed creation, stop and caller loss still trigger exact cleanup. The regression covers identity and listing failures, subsequent successful refresh, no premature removal and explicit final cleanup. All 60 USB cases pass; five maintained methods have at least 90% line coverage and every measured function is <=9 complexity. [Evidence](artifacts/2026-11-21-requested-fixes/t1540-verification.json). Only fake ADB endpoints were used.

## T1541 — display lease namespaces

A permanent native regression reproduced the collision between `T1541-Aa` and `T1541-BB` with disjoint ports. Display leases now use exact per-port names and a separate SHA-256 device-identity namespace. Same-device/port exclusion and stable-inode retention remain. All five lock tests and all 60 USB tests pass. Both maintained production methods exceed 91% line coverage; all 11 measured functions are <=9 complexity. [Evidence](artifacts/2026-11-21-requested-fixes/t1541-verification.json). Linux tests use private runtime directories. Windows native acceptance remains unmeasured. No installed binary or live lock namespace was changed.

## T1548 — camera target after Overview start

The failing-first GUI regression uses the real host camera Controller/session update with a private worker. Live edits and camera-tab Restart now overlay the admitted tablet onto a temporary profile, leaving saved drafts and Overview selection intact. Both ADB-serial and USB-peer cases pass, including changed mirror settings and original worker settlement. All 225 GUI cases pass; all three maintained methods exceed 95% line coverage and all measured functions are <=9 complexity. [Evidence](artifacts/2026-11-21-requested-fixes/t1548-verification.json). No camera or tablet was accessed. Windows native evidence remains tracked by T1271.

## T1550 — GUI activation wake

The production wrapper's unconditional timer failed the new idle regression. It now receives activation through one cancellable asynchronous socket reader and one coalesced pending bit. Window updates request focus only when that bit is set. The entry still retains the original abstract socket through native cleanup, and worker retirement remains exact. Tests cover malformed messages, coalescing, duplicate-reader rejection and original socket release.

All 225 GUI cases and both actual-entry/relaunch cases pass. Ten maintained production methods reach at least 80% line coverage, including 100% for the real main entry. All measured functions are <=9 complexity. A private Xvfb window observed zero update callbacks over 1.5 idle seconds; the restored old 250 ms timer produced six callbacks over the same interval. These are application callbacks, not kernel wake counters. [Evidence](artifacts/2026-11-21-requested-fixes/t1550-verification.json).

## T1551 — assigned-display repaint

Assigned and unknown display state now rely on existing status-worker change notifications. Camera/audio retain their active-state timers. The new idle regression failed before the fix; diff review also caught an overbroad draft edit to display buttons, which was reproduced by a separate permanent Stop-button test and reverted before commit.

All 225 GUI cases pass; the maintained presentation method exceeds 95% line coverage and all measured functions are <=9 complexity. Private Xvfb Overview measurement observed zero callbacks over 1.5 idle seconds, versus 18 with the old 100 ms timer restored as a test control. [Evidence](artifacts/2026-11-21-requested-fixes/t1551-verification.json). No resource ownership changes; Windows native presentation remains unmeasured under T1271.

## T1496 — generated display EDID identity

The failing-first helper regression confirmed that equal-geometry instances shared an EDID and cache path. Generated serials now use instance+1, reject overflow and include the instance in cache filenames. Repeated use of one instance remains stable; checksums remain valid. A bounded serial test covers 1024 consecutive instances plus the maximum accepted value, and X11 contract fixtures resolve equal-geometry outputs by distinct serials while retaining ambiguity rejection.

All 18 EDID cases, 36 helper cases and 14 X11 mapping cases pass. Five maintained production methods reach at least 80% line coverage; all measured functions are <=9 complexity. [Evidence](artifacts/2026-11-21-requested-fixes/t1496-verification.json). Tests generated files in private directories and started no EVDI/GPU session. Explicit custom EDIDs retain their supplied identity.

## T1560 — unavailable decoder queries

Failing-first Android and Rust regressions confirmed that unavailable inventory escaped onto the wire and was accepted as an empty capability set. The current Android request now retries with 100 ms–2 s backoff, cancels on replacement/disconnect, and never publishes busy or failed queries. The host rejects unavailable markers and unknown report fields; a genuinely completed empty inventory remains valid. The existing held-query regression now requires successful recovery after original retirement, retaining its stale-socket assertions.

All 17 Rust negotiation cases and 52 Android cases pass. Eight maintained Android methods/callbacks have 100% native JaCoCo line coverage; all 223 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1560-verification.json). The existing request job owns the bounded retry state; native query retirement remains independently owned. No device was accessed.

## T1585 — camera parser frame boundary

Confirmed by a private real-FFmpeg probe and a failing-first pipeline regression: one complete picture remained buffered until its successor. The decoder input now appends a six-byte access-unit delimiter after picture packets, leaving configuration-only packets unchanged. This supplies the frame boundary recognized by [FFmpeg's H.264 parser](https://github.com/FFmpeg/FFmpeg/blob/n6.1.1/libavcodec/h264_parser.c#L113-L120). The same single-picture regression now receives a frame before EOF or any successor.

All 126 camera cases pass, including three new regressions covering a real decoder, 512 NAL-header/prefix combinations, truncated prefixes and pipe failure. Both maintained production methods have 100% line coverage; all 13 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1585-verification.json). The allocation-free scan borrows one bounded packet and adds six static bytes under the existing write deadline. Original decoder retirement is unchanged. FFmpeg can diagnose the trailing metadata-only access unit at EOF; it produces no extra picture. No live camera or sensor-to-call latency measurement was used.

## T1561 — display worker recovery

A failing-first regression confirmed no restart after worker failure. Remote display now retains current demand and retries from 500 ms to 30 seconds, resetting after connected status. Each retry still passes through the original application-owned lane; held native retirement prevents replacement. Stop, surface/routes replacement, generation exhaustion and replacement by another Activity retain their cutoffs.

All 44 Android cases pass, including exact held-retirement recovery and deterministic backoff/reset/client-loss checks. Ten maintained methods/callbacks have 100% JaCoCo line coverage; all 155 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1561-verification.json). The Activity owns one cancellable retry job; the existing lane owns native cleanup. No physical tablet or process was disturbed.

## T1584 — audio backlog recovery

Failing-first Rust transfer/playback and Android playback regressions confirmed stale backlog after stalls. Consumer calls now permit one extra 10 ms jitter block; a larger backlog is trimmed to the configured target before consumption, with one discontinuity and reset partial offset/phase. Defaults and native clock-rate correction are unchanged. Existing overflow assertions now require the newest target window; the independent full-window resampling fixture explicitly selects a 200 ms target and retains its original sample/count assertions.

All 52 Rust audio cases and 14 Android audio cases pass, including 475 Rust and 285 Android target/backlog combinations. Synthetic 60 ms stalls produced 6 blocks initially and 8 on subsequent cycles; catch-up retains 4 before consuming one, leaving 3. Five maintained methods have 100% line coverage; all 120 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1584-verification.json). Existing queue owners discard their own bounded blocks; no native owner or queue is added. Physical audio latency was not measured.

## T1562 — rendered acknowledgements and UI isolation

Failing-first API-27/34 regressions confirmed replaceable receipt overflow and callbacks running on the caller dispatcher. Both IPC boundaries now retain one conflated receipt independently of the ordered-event capacity. The worker encodes once; the receiving Binder callback decodes once. Consumption runs on the default dispatcher, with presentation still passed to existing UI owners. Ordered failure/end events retain priority and exact native retirement is unchanged.

All 100 Android cases pass. In each paused-dispatcher case, 1,000 offered receipts reached the latest acknowledgement with zero additional UI callbacks and zero UI report callbacks. Seventeen maintained methods/callbacks have 100% JaCoCo coverage; all 281 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1562-verification.json). These are host JVM dispatch measurements; physical Android UI CPU-time acceptance remains T1631. The bounded immutable mailbox records own no native resources.

## T1607 — empty EVDI watchdog backoff

The permanent regression failed at the second empty recovery. Consecutive empty recoveries now double the interval from 250 ms to one second; notifications, immediate readiness and non-empty grabs restore 250 ms. Lost-event recovery and same-tick fallback coalescing remain. Sixty simulated idle seconds produce 60 recovery grabs after backoff; this is a deterministic harness count, not a live kernel measurement.

All 71 helper cases pass, and the dedicated new case passes ASan/UBSan. Seven maintained C methods reach at least 81.25% line coverage; all 235 measured C/Rust functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1607-verification.json). The existing capture context owns one bounded integer, with no new native lifetime. Live kernel-log and wakeup comparison remains T1632.

## T1571 — notification permission and Stop access

Failing-first manifest and UI regressions confirmed the missing permission and controls on API 27/34. The manifest now declares POST_NOTIFICATIONS. Tablet audio settings offer its runtime request on Android 13+ and Android notification settings on both versions. Denial leaves sharing available and explains that local Stop is available by reopening Blent. Resume refreshes OS notification availability; enabling notifications grants no media consent.

All 40 Android cases pass, including grant, denial, settings intent, resume changes and original service Stop ownership. All 25 maintained methods/callbacks have 100% JaCoCo line coverage; all measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1571-verification.json). The composition removes its lifecycle observer on disposal. No installed APK or physical service notification was changed.

## T1552 — external configuration refresh

The failing-first frame regression reproduced stale tablet-written settings. The existing status worker now reads a bounded validated snapshot and wakes the window on semantic change. The UI merges unsaved edits onto that snapshot and advances its Discard baseline. Read timestamps prevent observations started before a completed Save from overwriting it; timestamps alone never cause repaint. Missing, malformed and oversized files retain the current draft.

All 229 GUI cases, four storage cases and both private actual-entry cases pass. The storage tests include invalid UTF-8/TOML, non-regular files, a writerless FIFO and five capacity boundaries through 1 MiB+1. Nine maintained production methods reach at least 90% line coverage, including the actual GUI constructor; all measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1552-verification.json). The existing worker owns one bounded file read; the shared status owns one immutable snapshot. No thread, live session or persistent configuration was added or changed by verification. Windows native acceptance remains T1271.

## T1509 — visible process settlement

Three failing-first regressions confirmed silent pending cleanup, early PID removal and swallowed later signals. The process registry now retains bounded admission source locations and logs grouped pending counts immediately and every five seconds. The daemon entry owns PID and signal guards outside the runtime workflow. Repeated termination signals remain observable and explicitly report continued retirement; no deadline releases native debt.

Five registry-filter cases, 22 daemon-filter cases and seven integration cases pass. The actual daemon fixture holds fake route removal past five seconds, observes the entry-drain log and exact PID, sends another termination signal, then releases cleanup and verifies route/PID removal. Five additional private CLI invocations preserve passive/invalid-command behavior. Ten maintained methods meet at least 84% line coverage; all 162 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1509-verification.json). Location records share the existing 12,421-admission bound; each reporting map is temporary and bounded by the same count. Installed processes and routes were untouched.

## T1510 — daemon session failure

Two failing-first regressions confirmed ignored primary-worker and extra ADB-session failures. The daemon now observes the primary state and one monitor failure publication, treats unexpected channel loss as failure, logs the cause, closes admission and runs original cleanup before returning the error. The service's existing on-failure restart can then act. A normal shutdown already ready at the same time takes precedence. Extra ADB failures are published before the monitor awaits native retirement.

All six targeted regressions, 105 selected daemon/monitor cases and three actual-entry cases pass. Tests use real session supervisors with an injected worker error, both pending and admitted ADB sessions, channel loss, all terminal state variants and held original cleanup. Seven maintained methods reach at least 88% line coverage; all 171 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1510-verification.json). An isolated current-object rerun with a fresh raw-profile prefix resolved a stale namespace-profile collision; missing counters were not counted as passing. Existing owners and admission bounds remain; one watch publication carries failure without owning native resources.

## T1518 — interrupted Android trust recovery

The failing-first restart regression confirmed permanent rejection after an interrupted revocation. Android now persists the complete intended snapshot and rolls it forward under the original exclusive lock. Incomplete intents still fail closed. Saved computers offers an explicit confirmed reset that retains the tablet identity, clears remembered approvals and resumes after interruption. A stale viewer/request or unsettled original owner cannot authorize reset.

All 107 selected Android cases pass, including seven publication fault transitions, seven recovery transitions, eight reset transitions and 88 truncated intents. All maintained methods/callbacks have 100% JaCoCo line coverage; all 452 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1518-verification.json). Intent and temporary records each have a 65,624-byte cap; the empty reset marker and existing stable lock add no live native owner. Successful recovery requires eventual filesystem success and original-owner retirement. Physical Android acceptance is tracked separately under T1634; installed sessions were untouched.

## T1635 — host trust recovery, part of T1597

The failing-first restart regression confirmed permanent rejection of interrupted host revocation. A complete bounded intent now rolls forward under the existing exclusive lock. Invalid intent remains untouched until explicit reset; reset preserves host identity and resumes before any trust load. The native removal adapter validates fixed children and retains stable locks.

All 46 selected host security cases pass, covering eight failed and eight crashed publication checkpoints, six recovery failures, six failed and six crashed reset checkpoints, 88 truncated intents and five unsafe child kinds. Nine maintained methods measure at least 91% line coverage; all 40 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1635-verification.json). Tests use private directories and exact child waits. Parent T1597 remains open for identity recovery, confirmed repair entry, startup behavior and integrated acceptance.

## T1636 — host identity recovery, part of T1597

The failing-first restart regression confirmed rejection of a complete original key. Recovery now validates that certificate/key and publishes it without generating or replacing a key. Invalid or conflicting records require explicit reset. Identity reset holds both stable store locks, removes all identity-bound approvals, and leaves a durable marker that blocks trust until recovery completes.

All 54 selected host security cases pass, including nine failed and nine crashed reset checkpoints, two interrupted recovery checkpoints, every truncated prefix of one generated identity record, mismatched keys, conflicting records and held original locks. Eight maintained methods reach at least 95% line coverage; all 54 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1636-verification.json). Tests use private native filesystem/process fixtures. Parent T1597 remains open for the confirmed repair command, startup behavior and integrated acceptance.

## T1637 — broker signals before startup, part of T1597

The failing-first native fixture showed SIGTERM killing a broker paused before state initialization. Signal streams now register eagerly before startup; pending termination survives until the original startup settles. A ready stop takes priority over server dispatch, followed by normal owned shutdown.

All 11 selected entry cases pass. Private native child fixtures now survive both SIGTERM and SIGINT during the startup gate, complete original initialization and retire the original listener. Four maintained methods have 100% line coverage; all 12 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1637-verification.json). No installed process received a signal. Uncatchable termination and power loss still use journal recovery.

## T1638 — prompt broker startup failure, part of T1597

The failing-first native fixture confirmed that an exited launcher consumed the full ten-second startup wait. Independent launches now return an original-child exit receipt from the existing bounded reaper. The connector reports that exit promptly when no elected owner remains; a competing owner still initializing retains the normal readiness deadline. Socket success has priority. Launcher exit is not treated as delegated-service cleanup proof.

All 17 host cases, 37 shared command cases and the existing entry reaping case pass. Fourteen measured methods (12 maintained and two adjacent adapters) reach at least 83% line coverage; all 75 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1638-verification.json). Receipt loss preserves original child ownership, and one optional watch slot per existing child admission bounds retained state. Shared Windows native acceptance remains T1271. Parent T1597 still owns the confirmed repair command and integrated acceptance.

## T1597 — confirmed host repair and broker integration

T1635–T1638 implement recoverable trust/identity storage, early signal ownership and prompt launcher-exit reporting. The Linux broker now exposes `--reset trust` and `--reset identity`, both requiring `--confirm-reset`. Trust reset keeps the certificate/key; identity reset also clears all bound approvals. Live locks, unsafe state paths, missing confirmation and unrelated run options reject the request. [Recovery instructions](../troubleshooting.md#interrupted-usb-trust-or-identity) describe both modes.

The permanent repair/restart regression failed before implementation. All 77 selected entry/settings/state/security cases pass, including rejection of damaged startup, trust-preserving repair and complete broker restart, explicit identity rotation after reset, live-owner refusal and invalid inputs. Three maintained methods have 100% line coverage; all 38 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1597-verification.json). Verification used private filesystem/process fixtures and the production broker composition with an absent inventory helper. Installed trust, processes and devices were untouched.

## T1622 — private AppImage extraction and actual cleanup

The native baseline corrects the original report: the pinned runtime shares a digest-derived extraction directory, but its musl `nftw` call receives a zero descriptor budget and performs no deletion. The sibling sentinel remains present. Shared ownership and absent cleanup are confirmed; sibling deletion is not. Restoring cleanup alone would introduce the reported deletion, so both changes belong to one lifecycle fix.

The source-built runtime now owns one private directory and exact AppRun child per invocation. It forwards catchable stop signals synchronously, retains files until original-child reap, checks the pinned root identity, reports real cleanup failures and uses a positive, explicitly bounded traversal grant. Failed extraction/fork cleanup has no admitted child. The production packaging recipe pins all source archives, includes patches/build provenance in corresponding sources, rejects excess directory depth and runs the actual-runtime sibling/signal/retirement fixture before assembly. The prior prebuilt runtime is no longer a production fallback.

The production recipe built all five dependencies and the patched runtime in a private workspace. Actual-runtime qualification and all 40 AppImage Python cases pass; the normal suite includes native lifecycle/fault tests. GCC ASan/UBSan and leak detection pass. All 27 maintained functions exceed 88% line coverage; all 123 measured implementation/test functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1622-verification.json). Cleanup assumes original-child retirement and successful filesystem operations; SIGKILL, host crash, explicit NO_CLEANUP or reported removal failure may retain disk state. Physical FUSE mounting and installed sessions were not exercised. T1630 and T1639 separately retain the discovered upstream argument/path bounds work.

## T1630 — extraction argument terminator

A bounds-instrumented actual runtime passed explicit-flag extraction but trapped before AppRun in environment-only extraction. The argument array now reserves its NULL terminator. Packaging qualifies both invocation modes against the ordinary runtime and a separately built bounds-instrumented runtime; the latter is a test artifact. Both pass from freshly extracted pinned source with the production patches. Seven recipe/native unit cases pass; all three maintained recipe functions have 100% line coverage and all measured functions stay <=9 complexity. [Evidence](artifacts/2026-11-21-requested-fixes/t1630-verification.json).

## T1639 — runtime path boundaries

The actual-runtime oversized TMPDIR regression failed before correction. Fixed-size entry, environment and portable-path copies now use explicit capacity checks; readlink results reserve their terminator. The runtime and its bounds-instrumented test variant build with `-Wall -Werror` and pass all entry/lifecycle fixtures, including a 4092-byte executable path whose portable suffix exceeds the limit. Eight normal recipe/native test cases pass, including 600 bounded length/capacity combinations. ASan/UBSan and leak detection pass. All five maintained Blent functions have 100% line coverage; all 37 measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1639-verification.json). The adapter stores no views or heap allocations; input strings and bounded output storage belong to each containing call. This is boundary validation of the patched paths, not an exhaustive audit of upstream FUSE/SquashFS code.

## T1633 — accept failure retry

Both real descriptor-exhaustion regressions failed before correction; the video fixture observed 508 failed accept attempts over four polls before the intended 100 ms retry boundary. Input and video listeners now share a cancellation-safe retry deadline. Idle listeners retain their event-driven behavior, successful accepts clear the deadline, and stop/native completion remains selectable during backoff.

All 36 selected input/stream tests pass. The final three native cases also pass after adding input recovery: both servers recover when descriptors return, video stop bypasses backoff, and cancelling input retires its original native owner. All three maintained methods exceed 88% line coverage; all nine measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1633-verification.json). The only added retained state is one optional deadline per server. Windows native acceptance remains tracked under T1271.

## T1640 — KWin rejection settlement (part of T1508)

The private native service reproduced the bug: its negative property reply left the old port permanently incomplete. KWin writes now use a typed, bounded property request on the original retained worker. The worker resolves the service's unique identity and observes the original call's reply. Positive and negative replies from that service settle; broker errors proving rejection before dispatch also settle. Lost transport, a foreign reply or broker NoReply remains uncertain. Caller cancellation/deadline does not retire the original worker; port reuse requires its join.

All 62 selected native-command, KWin, OSK and input-mapping cases pass. The existing cancellation/deadline and OSK ordering regressions now exercise native writes. All 17 maintained functions exceed 88% line coverage; all measured functions have complexity <=9. [Evidence](artifacts/2026-11-21-requested-fixes/t1640-verification.json). Test-only command adapters retain the existing isolated frontend fixtures; production property writes no longer invoke busctl/qdbus. The input/OSK admission funds one task, one connection and one owned request; each text field is limited to 4096 bytes. No live desktop or installed process was changed. T1508 remains open for X11 acknowledgement and integration acceptance.

## T1641 — original X11 acknowledgement (part of T1508)

The permanent private-Xvfb regression failed on the old CLI port: an absent output prevented retirement and later mapping. Production writes now resolve the exact device name and active output on one native connection, validate the matrix property and observe the original XIChangeProperty check. Pre-dispatch rejection and original server success/error settle. Transport loss remains uncertain; cancellation retains the original worker through its actual join. Read-only discovery/readback still uses CLI tools.

All 70 selected native-command, KWin, OSK and mapping cases pass, including real server rejection followed by success, transport-loss retention and cancellation while the server is held. All 26 maintained functions exceed 82% line coverage; all 116 measured functions have complexity <=9. Rotation/reflection tests cover all 16 valid combinations and the complete 65,536-value flag domain. [Evidence](artifacts/2026-11-21-requested-fixes/t1641-verification.json). Production compilation and formatting pass. No installed process or physical device was changed; parent T1508 still owns integrated input/payload/process retirement acceptance.

## T1508 — integrated input retirement

T1640 and T1641 replace generic CLI mutation exit classification with original native reply evidence. The parent integration test now exercises the actual input lifetime, payload retirement and process registry together in a private Xvfb/D-Bus subprocess. Both X11 and KWin rejection permit successful later requests. A cancelled caller leaves its blocked original worker owned: payload and lifetime proofs stay incomplete, identity reuse fails and process drain waits. Releasing that original worker allows cleanup, drain and identity reuse.

All five selected input lifetime/payload tests pass, including this integration case. All eight new test functions have complexity <=9; prerequisite artifacts retain production coverage and failing-first regressions. [Evidence](artifacts/2026-11-21-requested-fixes/t1508-verification.json). No forced shutdown deadline or kill escalation was added. Genuine lost acknowledgements remain owned; this fix removes permanent quarantine caused merely by an acknowledged rejection. All 25 requested findings have local fixes. Hardware acceptance and unrelated ledger findings remain separately tracked. T1629 was subsequently explained and fixed as recorded above.

## T1643 — Android reinstall

Rebuilt and installed both release and profile APKs on the connected RugKing tablet. Both signing identities match their installed predecessors; the release also passes the designated certificate/package verifier. Pulled installed APKs match both verified build hashes. Data was retained, the release activity is foreground and successive tablet render acknowledgements confirm display delivery after reconnection. Temporary signing properties were removed. [Evidence](artifacts/2026-11-21-requested-fixes/t1643-verification.json). Native fault-injection acceptance T1631/T1634 remains separate.

## T1642 — desktop reinstall and live reload

Installed all four optimized Rust executables and both rebuilt C helpers into the existing local AppDir. All six installed hashes match staged builds, and every running Blent process image matches its installed executable. Original daemon and broker retirement were observed through their pidfds before replacement; the independent ADB server survived. Installed source metadata identifies `b560da7f` (the additional change after the build is test-only T1629).

The new daemon is active with zero automatic restarts. Successive tablet render acknowledgements confirm restored display delivery; native X11 touch and pointer mapping succeed. Tray activation opens one GUI process/window outside the daemon service, and repeating activation reuses it. [Evidence](artifacts/2026-11-21-requested-fixes/t1642-verification.json). This is a local SDK-backed installation, not a portable release. Physical fault-injection and static-display kernel measurements remain separately tracked; no camera/audio quality or universal leak-freedom claim is made.
