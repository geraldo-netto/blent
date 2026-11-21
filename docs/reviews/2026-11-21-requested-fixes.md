# Requested findings — 2026-11-21

Baseline: `58f58a32`. Work covers the 25 findings explicitly listed by the owner; confirmation and fixes are in progress. Existing unrelated TODO additions remain unstaged. No installed Blent instance or tablet session is restarted or changed. No push or deployment.

## T1615 — capture pause acknowledgements

Confirmed both missing transitions with permanent failing-first regressions: a new pause after capture is already quiescent, and a pause arriving while original native cleanup is held. Capture now acknowledges the current request after cleanup and while it remains parked. Three consecutive pause/resume transitions pass without a display/settings event. Held native cleanup still prevents acknowledgement; the existing cancellation and sibling-isolation assertions are retained.

All six quiescence cases pass. Five maintained production methods measure 100% executable-line coverage; 22 functions in the changed source/test scope have complexity <=7. [Evidence](artifacts/2026-11-21-requested-fixes/t1615-verification.json). No resource owner or queue is added: the capture worker retains the authoritative quiescent state until new capture admission, and exact helper/encoder settlement remains the prerequisite. Physical host-lock/display-sleep acceptance was not run against the user's session.

## T1629 — additional verification finding, open

The broader instrumented capture run passed 244 cases, failed one existing resume packet assertion and ignored two optional cases. The failed assertion required IDR plus codec configuration on the first received packet. Twenty normal and thirty instrumented isolated reruns pass; added generation diagnostics retain the original assertions. The cause is not established, and passing reruns do not resolve it. It remains in TODO.md for an explained reproduction and baseline comparison.

## T1489 — VAAPI quality option

Confirmed against installed FFmpeg option tables and [FFmpeg 6.1.1 rate-control source](https://github.com/FFmpeg/FFmpeg/blob/n6.1.1/libavcodec/vaapi_encode.c#L1807-L1816). VP9/AV1 VAAPI now receive `-global_quality`; H.264/HEVC retain `-qp`. The configured value is passed unchanged and the documentation distinguishes codec quantizer scales. The permanent regression failed before the fix and now covers all five VAAPI profiles at minimum/default-example/maximum values. All 14 existing encoding unit cases pass. Both maintained production methods have 100% line coverage; all 26 measured source/test functions are <=9 complexity. [Evidence](artifacts/2026-11-21-requested-fixes/t1489-verification.json). No hardware encode was run.
