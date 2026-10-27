# T1085: truthful retained-intent status and report cutoff

Camera/audio status distinguishes paused intent, active startup and pending cleanup, while old report permits cannot publish after replacement. Hosted tests and 4,096 bounded histories passed; a status change is not proof of native retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The independent portable auxiliary_activity::Phase schema distinguishes retained
Paused intent, admitted Starting attempts, Retiring work and Finished intent.
Retiring never certifies resource cleanup. CameraState/AudioState add Paused and
is_active, retaining Stop while waiting and while cleanup is pending. Camera
waiting-for-tablet video remains distinct from waiting for host availability.
Headless GUI checks keep paused camera and both audio Stop controls usable,
reject duplicate audio Start and preserve passive saved preferences.

auxiliary_work::run_observed reports Paused/Finished only after the previous
attempt and independent native settlement. Retiring precedes waiting for the
original work and native fence. Reporter panics become explicit errors: work and
settlement still complete before return, and no replay follows failure. Before
admission, a reporter failure creates no attempt. Both natural completion and
manual cutoff retain their actual native settlement obligation. Existing run
uses the same implementation with a borrowed no-op reporter. No callback is
retained in background work; callback-owned storage stays with the caller.

Report scopes separately fence publication permission. One workflow Owner holds
one current Permit; each begin seals the exact previous permit before creating
a fresh one. Revoke/Drop waits for its in-flight publication critical section,
then permanently closes that same metadata. Retained aliases never become current
on resume; no counter wraps and old Arc references keep their exact old bool.
The current-owner lock serializes begin/revoke, including concurrent callers.
Permit callbacks are synchronous short publications, cannot await or re-enter
the related owner, and must eventually release borrowed watch guards. Poison
recovery does not reopen a closed permit. Uncertain native resources remain
owned and funded by their independent original owners; sealing is no resource
retirement certificate. The source/runtime/native ownership contracts remain
T1081 and the existing native adapters.

Camera/audio scoped reports share the original authoritative status channels and
audio completion Tracker. All writes, including camera previews, require their
original permit. Camera channel senders are private; callers use a constructor,
owned read snapshots and gated preview publication. Root control reports retain
supervisor authority. Each native attempt must receive a scoped report, and the
workflow must revoke it before reporting cleanup/waiting and before replacement.
Production camera/audio availability wiring remains T1082/T1083; interface
readiness here does not establish runtime resume or physical acceptance.

Storage is one current scope per workflow plus caller-owned retained aliases;
there is no internal history, queue, callback retention or cycle. Each permit
owns only its mutex/bool metadata; snapshots own their copied strings or immutable
preview Arc. Previews retain the existing 160-by-160 RGBA bound. Rejection drops
incoming stale frames instead of storing them. Closing publication does not
discard original native proofs or release their admission. Eventual metadata
cleanup requires alias release; eventual native cleanup requires actual original
native evidence, callback quiescence and fair retained-owner polling. No native
abort, process death or unrestricted safety claim follows from these hosted tests.

Twelve new permanent normal-suite cases cover phase truth, initial paused Stop,
held cleanup, no early resume, last old callbacks during retirement, retained old
reports after resume, stale frame release, independent native proof retention,
reporter panic at every phase, natural completion, owner loss, poison recovery,
in-flight publication cutoff and both headless audio directions. The independent
bounded model explores all 4,096 six-step histories over begin/revoke/publish/
owner-loss, retaining up to six old aliases. Its oracle permits only the most
recent live alias to write and forbids any writes after owner loss. All model
objects are owned within one history; metadata is reclaimed between histories.
The model excludes unrestricted callbacks, native resource completion and all
OS interleavings; separate actual-thread tests hold a publication across cutoff.
A deliberate missing-original-cutoff control fails both a concrete regression
and the exhaustive model, which reports an old alias write after owner loss.

The original broad normal suite passes 1,897 cases, with seven existing opt-in
benchmarks ignored. The additional permanent model passes its normal suite. The final offline LLVM collection includes the
model and all current source hashes in a fresh exclusively owned target. Initial
native measurement passed all 36 selected production functions individually:
minimum 87.5%, with the CLI run and both GUI button functions at 100%. Final
counters/logs/object/source attestations are retained alongside this document.
No branches, source functions, thresholds or tests were excluded to raise scores.
The selection includes all new/changed function bodies and a few unchanged
wrapper methods sharing the same names. Source audit covers 405 functions,
maximum estimate 9, repository counter and cached tree-sitter-rust 0.24.2/
tree-sitter 0.25.2. This is not native SonarQube evidence; T1019 remains blocked.
Twelve architecture checks and portable no-default-features compilation pass.
The latter exposes three existing unused batch warnings, tracked separately as
T1087. No network transfers, installations or physical actions occurred.

Final native collection: 1898 cases passed, 7 existing opt-in benchmarks
ignored. All 36 selected production functions pass individually;
minimum 87.5%. Final source audit: 405 functions, maximum estimate 9.

</details>
