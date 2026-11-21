# Project review — 2026-11-21

Baseline: `27352839`. Review in progress; this is not a whole-project correctness certification. No installed Blent process or device session is changed.

## T1476 — partial live-camera effects

A sensor update could succeed before an encoder failure, while the deduplication cache still described the previous settings. Reverting then returned success without restoring the sensor. Invalidate the cache before native effects; only complete successful application establishes a new cached command.

Permanent regression: `CameraLiveTuningTest.t1476_partialNativeFailureCannotAcknowledgeARevertWithoutRestoringTheSensor`, failed before the fix with actual exposure 100 versus requested 0 on API 27 and 34; passed afterward. The first attempted reproduction used an unsupported-exposure fixture and was corrected before collecting this evidence. All 10 cases in CameraLiveTuningTest pass. JaCoCo 0.8.15 measures all four production methods at 100% executable-line coverage. Repository Kotlin complexity checker: maximum 4 in the production class; every measured test method/callback <=3.

The original capture worker owns one bounded cached command and receipt. Synchronous native calls complete on that worker before resource retirement. Cache invalidation neither releases native objects nor changes worker admission, retirement or reuse. Unsupported validation leaves known native state intact; uncertain effects force a later revision to reassert controls.

## T1478 — embedded notices

The About inventory omitted 17 checked-in camera-color documents. Added their verbatim compile-time inclusions. Existing permanent T1277 inventory/content regression failed at 81 authoritative files versus 64 inclusions; it now passes with 81 exact matches. No runtime method, allocation or native ownership changes. T1463 upstream notice qualification remains open.
