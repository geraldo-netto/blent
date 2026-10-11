# T1004: joined fixture input stays inside actor advancement

The intermittent T1000 admission timeout came from a fixture mismatch, not a
confirmed production bug. The fixture drained incoming host Offer/Commit frames
immediately from the test thread. Commit could interleave after local advancement
but before publication: offer becomes invisible, local route is not admitted yet,
and false AudioClosed precedes AudioRoute for the same lease. Original presentation
correctly refuses to revive a closed owner; consent disappears before native start.

Controlled event/cancellation/endpoint diagnostics reproduce that exact sequence
with no endpoint failure or cancellation. Production ConnectionSessionRunner
readComplete delivers incoming frames inside advance, followed by the same turn's
local-resource advancement. The fixture now queues host Offer/Commit/retirement
frames until that turn. Host mux activation and native ownership stay independent.
No production behavior, authorization, timeout or retirement assertion is relaxed.

Permanent API 27/34 offer/commit regressions fail before the fix, then pass unchanged
for every resource kind. Bounded 32-cycle duplex admission includes pending camera
withdrawal, exact sibling display/audio authority and authenticated PCM; both SDKs
pass. Existing joined camera/microphone/speaker, Stop, failure/death, held cleanup,
fresh replacement and T1005 recipient regressions remain in the ordinary suite.

Fresh offline focused collection: 52 tests; full Android suite: 1,959 tests, no
failures/errors/skips. All 2,741 production methods pass the individual native 80%
executable-line gate. Changed authored fixture complexity: 207 methods, maximum 6.
Sources remain immutable from snapshot through build and native check. Inventory,
JaCoCo, individual counters, exact source hashes and red/green diagnostics retained.

No downloads, real USB/ADB, installation, live host services or tablet changes.
Native device/death evidence is controlled synthetic evidence. Physical acceptance
remains postponed. Pre-existing unrelated TLS edits remain unchanged and unstaged.
