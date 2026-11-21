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

## T1479 — production worker lookup in tests

Cargo 1.100 resolves test executables under `debug/build/<package>/<hash>/out`; fixed parent traversal found no production encoder worker. The test-only resolver now locates the owning Cargo profile via its deps directory and requires the separately built production binary. The script prerequisite fixture uses an explicit production executable path. Existing T819 native worker tests failed with ENOENT before the fix and pass afterward; permanent T1479 covers both artifact depths, missing binary and outside-profile rejection. All 10 offline-runner Python tests pass. This changes test composition only, preserving production spawn, original child/reader retirement and native identity checks. The new resolver and regression satisfy the <=9 complexity gate.

## T1481 — pending fixture ownership

Three permanent GUI tests failed because fake camera/audio starts discarded their only Acknowledger while asserting Pending. Fixtures now retain one acknowledger per backend until stop/drop, matching production settlement semantics. The direct owner-state test retains its original acknowledger through pending assertions. The same T1273 owner/workspace and T979 exact-transport regressions pass. No production behavior changed; completion retention is bounded to each fixture backend, with stop/drop as cleanup sponsor. All measured changed test methods remain <=9 complexity.

## T1482 — monitor fixture namespaces

Read-only lock inspection confirmed the live daemon holds display-route stripes 112/113, which the fixtures also selected for ports 18000/18001. Each affected monitor fixture now executes in a private child with an explicit mode-0700 runtime/config/home directory. No global parent environment mutation, live lock removal or production lock relaxation occurs. The initial private-directory attempt was rejected for its default 0775 mode; the explicit private mode fixes that fixture error. All 78 monitor cases now pass, including the previously failing credential/launch/forwarding families. Child completion precedes temporary-directory cleanup; test bodies retain their original route settlement checks.

## T1483 — required warning gate

Clippy 1.100 rejected the new brightness index loop, three divisibility predicates, a redundant USB identity conversion, USB test-module placement and an OSK test error constructor. Equivalent iterator/predicate/error forms and module ordering restore that gate without suppressions. The normal three camera-probe compile checks pass after also composing the T1485 fixture. All 32 common camera tests and all 53 selected USB library tests pass. Exact-object LLVM coverage measures curve, placement, point, plane and sessions individually at 100%; the repository complexity checker reports no >9 function in the changed scope. The LUT remains a stack-owned 256-byte array, plane processing retains caller-owned bounded slices, and session enumeration retains its existing owner/borrow boundaries; no allocation or native lifetime policy changed.
