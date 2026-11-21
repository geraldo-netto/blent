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

## T1485 — camera pool integration composition

The standalone pool target failed compilation because its production source now depends on camera_live. It now imports that production module with an explicit test-module path. The existing controller/session integration regression moved unchanged to camera_control/live_tests, its actual owner, so the independent pool fixture does not import the entire controller implementation. All 12 pool target cases pass, including immutable retained frames and allocation checks; the moved controller regression also passes. Clippy compiles the integration target with warnings denied. No production API or runtime behavior changed and no permanent regression was removed.

## T1484 — startup deadline fixture

The native missing-broker test allowed 3 seconds while production settings admitted a 10-second startup interval. Independent systemd launch exposed this mismatch because successful launcher creation does not prove successful broker startup. The fixture now explicitly selects a 100 ms startup deadline and retains its 3-second observation bound. The same permanent T894 failure/retirement/no-files assertions pass. Production deadlines and process ownership are unchanged.

## T1480 — current ADB identity and cleanup fixture

The CLI retry test counted physical-identity queries as Activity launches and its fake failed creator supplied neither an observable mapping nor settlement evidence. The fixture now returns a stable identity, records only actual launch shell requests in launch assertions, models a lost acknowledgement after an observable mapping creation, and awaits original route retirement before retry and final cleanup. A private mode-0700 child runtime isolates native locks. The internal RouteOwner retirement method is crate-visible for this existing CLI owner fixture; its implementation is unchanged. The original permanent T144 failed before correction and now passes its failure/backoff/two-mapping recovery/exact-launch assertions. Unknown native outcomes remain retained in production.

## Review scope and limits

Repository-wide checks cover the supported Rust, Kotlin, C, Python and shell source inventories, the Linux Rust workspace, Android debug JVM suite and tooling/packaging regressions. Manual review follows behavior across domain boundaries: GUI activation and configuration persistence; live camera command/receipt, brightness, transforms and frame freshness; audio/connection admission and retirement; display ADB identity/mapping ownership; USB helper streams and native frame exchange/rings; pairing, trust persistence and wire bounds; service templates, installation and CI composition. Windows entry/input boundaries were inspected as preview code. Historical review artifacts were consulted as evidence, not treated as current implementations.

The baseline inventory contains 12,970 tracked files, predominantly historical documentation/artifacts. This is an inventory count, not a claim that every file or line received manual inspection. Source inspection and automated tests cannot establish absence of all bugs or leaks. Existing Windows native acceptance blockers, subjective face-image quality acceptance (T1452), upstream distribution notice qualification (T1463), and declined platform/measurement scopes retain their recorded dispositions.

Memory/resource review examined authoritative worker/session ownership, bounded commands and frame leases, admission rejection, partial native failure, cancellation, completion acknowledgements and cleanup before reuse. The camera fixes add no native resource owner or queue. Frame-ring retirement continues to depend on original readers/writers and generation checks; deadline expiry is not treated as native settlement. Existing native C ASan/UBSan fixtures run through the tooling suite; no new allocator or physical-erasure claim is made.

Complexity toolchain: repository checker with Rust syn 2.0.119, Kotlin PSI 1.9.20, tree-sitter 0.25.2, tree-sitter-bash 0.25.1 and tree-sitter-c 0.24.2. Final supported inventory: 30,640 measured functions, zero above 9. JavaScript/PowerShell omissions remain explicitly declined under T1397/T1398; this count is not evidence for those languages. Changed production coverage uses JaCoCo 0.8.15 and cargo-llvm-cov 0.8.7 with exact current LLVM test objects.

## T1487 — included source formatting

The authoritative `scripts/format-rust.py` also traverses the Linux supervisor include tree, which ordinary cargo fmt omits. It corrected formatting of the encoder test resolver and preexisting tray regression. Its --check gate passes; only whitespace/layout changed.

Additional T1480 measurement: the unchanged internal retirement implementation, whose visibility was widened for the owning CLI fixture, has 6/7 executable lines covered (85.7%), above the per-function 80% requirement.

## T1486 — unresolved intermittent speaker regression

The first uncontended full Android run completed 2,869 cases with one API-27 failure: the speaker offer was null before the blocked-renderer test requested Stop. The affected class then passed; 20 isolated invocations covering both API levels passed all 40 cases; a second full run passed all 2,869 cases. The original native-retirement/display-independence assertions remain unchanged, with additional state diagnostics on the failing offer assertion. No root cause or production defect is established. T1486 remains open; passing reruns do not resolve the observed failure. Permanent evidence records the original failure and bounded rechecks.

## T1488 — private GUI startup fixture

The permanent T078 startup smoke test failed before creating a window because its shared config/runtime fixture precreated `blent/` with umask-dependent mode 0775. The fixture now explicitly sets both owned directories to 0700; production singleton validation remains unchanged. The same startup/accessibility/update/close regression passes afterward. The two fixture functions measure complexity 1 and 2; the authoritative formatter passes. This adds no production method or resource owner.
