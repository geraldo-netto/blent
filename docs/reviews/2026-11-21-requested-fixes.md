# Requested findings — 2026-11-21

Baseline: `58f58a32`. Work covers the 25 findings explicitly listed by the owner; confirmation and fixes are in progress. Existing unrelated TODO additions remain unstaged. No installed Blent instance or tablet session is restarted or changed. No push or deployment.

## T1615 — capture pause acknowledgements

Confirmed both missing transitions with permanent failing-first regressions: a new pause after capture is already quiescent, and a pause arriving while original native cleanup is held. Capture now acknowledges the current request after cleanup and while it remains parked. Three consecutive pause/resume transitions pass without a display/settings event. Held native cleanup still prevents acknowledgement; the existing cancellation and sibling-isolation assertions are retained.

All six quiescence cases pass. Five maintained production methods measure 100% executable-line coverage; 22 functions in the changed source/test scope have complexity <=7. [Evidence](artifacts/2026-11-21-requested-fixes/t1615-verification.json). No resource owner or queue is added: the capture worker retains the authoritative quiescent state until new capture admission, and exact helper/encoder settlement remains the prerequisite. Physical host-lock/display-sleep acceptance was not run against the user's session.

## T1629 — additional verification finding, open

The broader instrumented capture run passed 244 cases, failed one existing resume packet assertion and ignored two optional cases. The failed assertion required IDR plus codec configuration on the first received packet. Twenty normal and thirty instrumented isolated reruns pass; added generation diagnostics retain the original assertions. The cause is not established, and passing reruns do not resolve it. It remains in TODO.md for an explained reproduction and baseline comparison.

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
