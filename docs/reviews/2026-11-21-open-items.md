# Open-item fixes — 2026-11-21

Owner authorized all 112 items marked open at the start of this pass. Existing blocked and declined dispositions remain unchanged. Work is ongoing; this report records completed increments, not completion of the backlog. Source tests do not imply native platform acceptance.

## T1596 — stable USB inventory targets

Unchanged inventory scans retain physical-key target IDs and revision, while refreshing opaque backend payloads. Visible metadata changes advance revision and reject stale Connect requests; removal/reappearance allocates a new ID. Capacity, duplicate-key and counter exhaustion failures leave the original inventory intact. Original live links retain ownership independently of inventory changes.

All four new regressions failed before the fix. All 32 broker-control tests pass afterward, including original authority, tombstone and native-retirement tests. The exhaustion regression now requests a genuinely new target instead of requiring an unchanged target to consume an ID. All five maintained production functions have 100% executable-line coverage (cargo-llvm-cov 0.8.7); all 16 measured functions in the new inventory and affected boundary-test files have complexity <=9 (project Rust syn 2.0.119 checker). The controller owns at most the configured 512 inventory entries; replacement temporarily owns one additional bounded inventory and a key index, released on success or error. No native resource is created, released or reused by a scan.

## T1549 — queued media replacement

Camera/audio panels retain one validated replacement request after consuming the inactive original backend. The original owner and exact native fence remain authoritative until settlement; only then can the saved request create/admit its successor. Frame maintenance runs for hidden panels, with repaint polling only while a request is pending. Stop drops the queued request without claiming early retirement. Admission failures consume the request once and preserve the error, including immediately completed fences.

Six original delayed-retirement regressions failed before the fix. Two additional immediate-retirement feedback regressions failed against the initial implementation and pass after correction. All 42 selected GUI tests pass, including hidden-panel progress, cancellation, admission identity, validation and idle repaint behavior. All 17 maintained production methods have 95.65–100% executable-line coverage; all 283 functions measured in affected files have complexity <=9 (project Rust syn 2.0.119). Each panel owns at most one validated settings value, released on cancellation, admission/failure or panel destruction; the preexisting backend cleanup sponsor retains native work after close. This does not grant a new target authority before original retirement. Native Windows acceptance remains tracked in T1271.

## T1599 — visible USB broker diagnostics

The Linux broker writes tracing diagnostics to standard error, which its on-demand launcher inherits. The permanent black-box regression starts the actual broker binary with private state/runtime directories and a failing fake helper; it failed before the fix because the warning went only to stdout. It now observes the warning on stderr and an empty stdout, then terminates and reaps that exact child. The existing subscriber-startup failure test also passes.

The maintained Linux entry function has 88.89% native executable-line coverage in the unit binary; the black-box regression additionally exercises the running production entry. All six functions measured across entry/fixture files have complexity <=9 (project Rust syn 2.0.119). The unchanged unsupported-platform entry is outside this change. The fixture owns its child and temporary files, reaps on assertion failure, and never invokes a real accessory helper or installed broker.

## T1556 — current desktop button names

Conversion, encoder-worker, GPU and pipe-buffer help now names Save or Save & restart all displays, matching the actual controls. GPU wording makes its restart requirement explicit. Four existing settings tests pass; the pipe disclosure test now opens the help and verifies the Save instruction. All four maintained help-rendering functions have 100% line coverage; measured functions in affected files remain <=9 under the project Rust syn checker. This is text correction with unchanged settings behavior and resource ownership. Current guide corrections remain under T1591.

## T1519 — explicit connection-worker timeout failures

Own binding/startup timeout expiry becomes IOException, while caller cancellation and nested deadlines retain their original cancellation identity. Saved-trust observation has its own validated 1–60000 ms budget, default 5000 ms, independent of the startup budget. Expiry only ends observation; the original Binder, binding and blocked IPC jobs retain existing retirement obligations. No resource is released based on timeout alone.

All six original API-27/34 reproduction cases failed before the fix. All 84 selected Android/JVM cases pass after it, covering expiry, budget bounds, caller cancellation, nested timeout identity, trust operations, blocked start/stop and ResourceLane settlement. Existing retirement tests use the dedicated trust budget and IOException classification while preserving exact original-Binder/native-call assertions. All 11 maintained methods/callbacks have 100% JaCoCo 0.8.15 line coverage. All 339 functions measured in affected Kotlin production/test files have complexity <=9 (project Kotlin PSI 1.9.20 checker). These are host-JVM/fake-Binder observations; installed APKs and physical Android sessions remain unchanged.

## T1591 — current guide corrections

Verified and corrected the cameras CLI spelling, remaining Windows route-cleanup tracking, Settings-tab navigation, default five-frame YUV storage accounting, platform entry/CLI source locations and desktop Save/Save & restart all displays labels. Removed the source map's stale claim that Debian fixtures remain. Android's actual Apply control is unchanged. Sources checked include the CLI grammar, GUI save label/priority controls, frame pool plus transform allocation and platform entry files. A focused scan finds none of the superseded phrases in the eight affected guides; diff whitespace checks pass. Documentation-only: no executable behavior or resource lifetime changed, and no artificial test was added.

## T1598 — inspectable state during USB discovery failure

The shared snapshot carries a mandatory inventory-availability flag. Discovery failure hides stale connection targets while preserving pending pairing, live links and saved peers; the GUI displays the failure and retains disconnect/forget controls. New Connect commands require successful discovery. Each admitted client can retain one initial Inspect request until the first scan outcome; client closure clears it. Repeated scan failures do not manufacture revisions, and successful empty inventory recovers normally. Obsolete wire snapshots without the flag are rejected.

Three independent regressions failed before the fix. Afterward, 98 host tests, 27 shared-contract tests, 26 GUI tests and two actual-broker fixture tests pass; the final five targeted server regressions also pass. Workspace all-target compilation, Rust formatting and whitespace checks pass. All 20 maintained production functions have 93.33–100% executable-line coverage. All 399 measured functions in affected Rust files have complexity <=9 (project Rust syn 2.0.119 checker). Pending inspection storage is bounded by admitted client capacity and released by first outcome or client closure; existing link/native completion ownership is unchanged. Fixtures use private directories and fake USB helpers, without touching installed sessions. Native Windows acceptance remains under T1271.

## T1623 — application-owned AppImage environment

The launcher and direct installer derive their actual application directory from their own canonical script paths. They discard inherited APPIMAGE unless inherited APPDIR resolves to that directory. Matching directory aliases remain valid. Registration from a foreign or absent APPDIR copies the Blent directory instead of another application's image. The extraction-lifetime fixture now supplies the current extraction APPDIR, as the real runtime does; publication fixtures still extract definitions before startup.

Three regression methods failed before the fix across 12 foreign/unset/empty/missing-directory cases. All 45 AppImage tests and 16 publication tests pass afterward. Both maintained shell functions have 100% native Bash line coverage; separate normal-suite runs execute AppRun with its POSIX-sh shebang. All 30 measured functions in affected source/fixture files have complexity <=9 (project tree-sitter shell and Python AST checkers). This change retains bounded environment strings and synchronous command substitutions only; installation locks, staged-file cleanup and original extraction ownership are unchanged. No installed application was changed.

## T1533 — validate raw-frame capacity before capture

Configuration sanitization clamps raw_slots to the shared transport's 2–MAX_SLOTS range; direct validation rejects values outside that range before publication or activation. The default remains four slots. Two permanent regressions failed before the fix and now cover every value 0–64 plus u32::MAX, checking both model validation and transport layout admission. All 25 selected model/raw-frame tests pass. The two maintained production functions have 86.49–100% line coverage; all 34 measured functions in the model file have complexity <=9 (project Rust syn 2.0.119 checker). This scalar validation allocates no native resource and does not change existing frame-lease retirement.

## T1531 — durable configuration rename

Unix configuration saves sync the containing directory after the synced temporary file is renamed. Success diagnostics and publication callbacks follow that sync; a sync failure returns an error without pretending the already-visible rename was undone. The injected publication boundary verifies exact ordering and error propagation. Two regressions failed before adding the sync call; all 25 storage tests pass afterward, including failure-before-rename and native directory-sync error cases. All three maintained production functions have 95.83–100% line coverage; all 40 measured functions in affected files have complexity <=9 (project Rust syn 2.0.119). The original transaction lock remains held through sync; directory/file handles are lexical owners and close on success or failure. Native Windows publication behavior is unchanged; these durability claims are Unix-scoped.

## T1532 — disclose unreadable configuration

Only a missing configuration is a silent default. Other read errors log the path, OS/encoding error and explicit fallback; malformed TOML retains its existing diagnostic. The independent Snapshots contract permits best-effort startup defaults, so recovery remains available while partial updates still refuse unreadable input. Loading leaves original bytes untouched. The new unreadable-file regression failed before the fix; all 27 storage tests pass afterward, covering invalid UTF-8, a directory in place of the file, missing/valid input and malformed TOML. The maintained load function has 100% line coverage; all 36 functions measured in affected source/test files have complexity <=9 (project Rust syn 2.0.119). The synchronous read and diagnostic borrow are scoped to the call, with no additional retained state or native resource.

## T1517 — retry unsuccessful update checks

The first check remains 30 seconds after startup. Valid responses restore daily polling; failures retry after 60 seconds, doubling to a one-hour maximum. Success resets that delay, and failure retains the last valid notice. The doctor's comment now matches the four-second curl transfer limit and five-second command deadline.

The injected-program offline-start regression failed before the fix and passes afterward. All 11 update tests pass, including original cancellation/child-retirement tests. A permanent bounded policy test checks all 4096 twelve-outcome success/failure sequences. The three maintained production functions have 81.25–100% line coverage; all 24 measured functions in affected update files have complexity <=9 (project Rust syn 2.0.119). One bounded duration is retained by the existing polling task; retries remain serial and use the original command child/collector ownership and cancellation cleanup. Tests use local programs and simulated clock advancement, without network access.

## T1539 — settle Wi-Fi-off observation failures

An off attempt first observes inventory without claiming a mutation. A proven observation nonstart reports ADB unavailable; failed or malformed inventory returns an error after the saved-address removal. Neither retries nor retains an endpoint cleanup obligation. Confirmed absence needs no disconnect. Once disconnect starts, original endpoint ownership remains until confirmed absence, even after caller cancellation or lost acknowledgement. A proven disconnect nonstart releases the off attempt. Retained cleanup emits only its first warning per episode.

The stalled-initial-read and repeated-warning regressions both failed before the fix. All 44 Wi-Fi tests pass, followed by five targeted final regressions including a real missing executable, saved-address/status behavior, and rejected versus acknowledged versus unknown disconnect outcomes. All ten maintained functions have 96.15–100% line coverage; all 55 measured functions in affected files have complexity <=9 (project Rust syn 2.0.119). Tests retain and settle the original worker before reporting failure, and verify replacement exclusion while removal remains unknown. Native command child/collector fences and the process-exit guard remain authoritative; timeout alone never releases a started disconnect. Native Windows acceptance remains tracked in T1271.

## T1545 — preserve valid non-ready ADB records

Current upstream [ADB state formatting](https://android.googlesource.com/platform/packages/modules/adb/+/HEAD/adb.cpp) includes detached; its [state contract](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/adb.h) identifies it as a known USB device not claimed by that server. Blent now accepts bounded lowercase state tokens instead of a closed whitelist. Only device is ready; other valid tokens remain present for endpoint ownership. Empty/malformed states, control characters, oversized tokens/diagnostics and duplicate identities remain rejected. Existing bogus-token fixtures now use malformed multiword states, while new tests prove that unknown valid tokens never settle removal as absence.

All three original regressions failed before the fix. Twelve selected inventory/cleanup tests pass, followed by seven final inventory tests including all 256 single-byte state-token boundaries and the 1024-byte permission-diagnostic boundary. All three maintained production functions have 100% line coverage; all 26 measured functions in affected files have complexity <=9 (project Rust syn 2.0.119). State validation borrows the existing bounded command output; no native resource or new retained state is introduced.

## T1649 — require an endpoint in connect success

Additional finding from this pass: the parser accepted bare connected without identifying an endpoint. Removed that unsupported branch; current endpoint-qualified success and already-connected replies remain accepted. The new regression failed before the fix; all 18 Wi-Fi policy/setup tests pass after retargeting the one old acceptance fixture to the current response contract. The maintained parser function has 100% line coverage; all 37 measured functions in affected files have complexity <=9 (project Rust syn 2.0.119). Reply processing remains call-scoped and does not alter native creator/retirement ownership. Endpoint normalization remains a separate T1544 fix.
