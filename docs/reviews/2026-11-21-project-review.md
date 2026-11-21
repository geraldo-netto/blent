# Project review — 2026-11-21

Baseline: `27352839`. Review in progress; this is not a whole-project correctness certification. No installed Blent process or device session is changed.

## T1476 — partial live-camera effects

A sensor update could succeed before an encoder failure, while the deduplication cache still described the previous settings. Reverting then returned success without restoring the sensor. Invalidate the cache before native effects; only complete successful application establishes a new cached command.

Permanent regression: `CameraLiveTuningTest.t1476_partialNativeFailureCannotAcknowledgeARevertWithoutRestoringTheSensor`, failed before the fix with actual exposure 100 versus requested 0 on API 27 and 34; passed afterward. The first attempted reproduction used an unsupported-exposure fixture and was corrected before collecting this evidence. All 10 cases in CameraLiveTuningTest pass. JaCoCo 0.8.15 measures all four production methods at 100% executable-line coverage. Repository Kotlin complexity checker: maximum 4 in the production class; every measured test method/callback <=3.

The original capture worker owns one bounded cached command and receipt. Synchronous native calls complete on that worker before resource retirement. Cache invalidation neither releases native objects nor changes worker admission, retirement or reuse. Unsupported validation leaves known native state intact; uncertain effects force a later revision to reassert controls.

## T1478 — embedded notices

The About inventory omitted 17 checked-in camera-color documents. Added their verbatim compile-time inclusions. Existing permanent T1277 inventory/content regression failed at 81 authoritative files versus 64 inclusions; it now passes with 81 exact matches. No runtime method, allocation or native ownership changes. T1463 upstream notice qualification remains open.

## T1477 — transactional live bitrate policy

`CameraRate.configure` previously committed limits/current bitrate before the native codec call. A failed call could make a retry skip the native write and falsely report success. The explicit native callback now runs before policy commit and reasserts unchanged targets after uncertain effects. The capture worker remains the single synchronous policy/native owner; no queue, allocator, detached callback or resource release was added. On rejection the previous limits, adaptive flag, current bitrate and observation history remain owned and unchanged.

Permanent T1477 regression failed with current bitrate 5000 instead of retained 3000 before the fix. It now passes, including retry and reverting after a native effect followed by failure. All 250 camera test cases pass (API 27/34 where configured). The existing exact-call pressure test now includes the initial 3000 kbit/s reassertion before its required 2250 kbit/s reduction. JaCoCo 0.8.15: configure, stream and six compiled callbacks each 100% executable-line coverage. Repository checker measured 186 methods/callbacks across the five changed files, maximum complexity 8.
