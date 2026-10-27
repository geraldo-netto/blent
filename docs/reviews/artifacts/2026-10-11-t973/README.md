# 2026-10-11-t973 — README

Android camera controls bind Start/Stop to the rendered original USB offer and keep permission separate. Status reports admission, not delivered frames; physical acceptance remains separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T973: production MainActivity renders original-owner USB camera consent through the independent CameraAccessControls interface. Legacy ADB controls remain the fallback when no USB camera offer exists. Local Start is available before admission; SDK permission and background intent remain separate. Callbacks retain the rendered offer, so queued stale taps cannot approve a replacement. Stop affects camera only. Status reports requests/admission, never claims delivered frames.

The permanent consent UI regression fails before behavior is implemented (the optional contract parameter was a test seam only), then passes unchanged on SDK 27 and 34. Full normal Android suite also retains real MainActivity joined controls, denied permission, original-owner policy, stale callback, background lifecycle, capture and actual isolated worker death/retirement fixtures. See test-summary.json for exact counts. All 2507/2507 authored production functions/methods pass their individual 80% native line gate. Changed complexity has 105 authored methods, maximum 5. Raw JaCoCo counters, source inventory/hashes and initial red proof retained. Physical camera-only debugging-off acceptance stays T858; host GUI selection stays T974. No downloads or live device operations.

</details>
