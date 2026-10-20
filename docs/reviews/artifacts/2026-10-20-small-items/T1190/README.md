# T1190: terminal native-deadline notifications

NativeDeadline now uses the independent GenerationFence. Begin reserves a
positive callback identity before creating a timer; exhaustion reports a visible
pending-cleanup status without allocating another job. Finish revokes without
arithmetic, cancels the original timer and clears status. Old expiry callbacks
cannot restore status after finish or alter another attempt. Notification
exhaustion never supplies evidence that the native owner retired.

Three permanent regressions fail before the fix: terminal begin creates a timer,
the final finished timer permits another admission, and an old MIN expiry aliases
the wrapped current attempt. The same assertions pass afterward. An independent
BigInteger oracle checks34 boundary seeds with20 begin/expire/finish sequences
each (680 sequences),33 negative seeds and actual timer cancellation joins.
Existing real-expiry, stalled resource and missing camera callback tests remain.
All16 selected tests pass with no skips.

JaCoCo0.8.15 reports100% executable-line coverage individually for all7 measured
owner method variants, including default construction and the separately
compiled timer callback. Cached pinned Kotlin PSI1.9.20 models26 functions,
maximum3. Native SonarQube remains unavailable under T1019; these model scores
are not native certification. Raw reports, source hashes and red/green logs are
retained. Use the offline Gradle/coverage invocation from adjacent T1180 evidence
with NativeDeadlineTest, GenerationFenceTest, ResourceLaneTest and
CameraStartupDeadlineTest as the selected classes.

One NativeDeadline owns one fence, one replaceable status and at most one timer.
Delay capacity remains1–60,000ms; revisions stay in0–MAX and never decrease or
reuse a grant. The existing monitor serializes begin/finish/expiry. A timer
captures only its original immutable grant and keeps the containing observer
alive until completion. Finish revokes before cancellation; late queued expiry
can neither access a native resource nor restore authority. Rejection retains no
fresh job. Tests retain original handles through actual cancellation/join,
including failure-finally cleanup. Eventual coroutine settlement assumes the
dispatcher executes cancellation; no timer completion or status is native
retirement evidence. Original decoder/camera owners remain separately funded
until their own finish/proofs. No native ownership, device, service, APK,
Windows/EVDI backend or network transfer was changed.
