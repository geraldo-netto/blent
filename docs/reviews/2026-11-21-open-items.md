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

### T1545 — wider USB fixture follow-up

The wider USB suite exposed one additional fixture still treating the valid token bogus as malformed. That existing regression failed, then passed after changing its malformed case to a multiword state and adding detached to its non-ready records. All 37 USB tests pass in the combined Wi-Fi/USB run. All 34 functions in the changed USB test file have complexity <=9. No production behavior or original malformed/duplicate/ownership assertion was weakened.

## T1544 — normalize connect-reply endpoint identity

A private native ADB 34.0.4-debian server confirmed the bug: a request for LOCALHOST:055637 returned connected to LOCALHOST:55637. The probe used its own filesystem server socket, temporary home/keys, a nonexistent --one-device serial and one simulated loopback adbd; all probe-owned sockets, thread and server process were stopped and joined. It did not address the installed server or physical tablet.

Connect success now validates both endpoints and compares the same normalized identity used for reservations and observations. Hostname case, port leading zeroes and canonical IPv6 spelling match; different endpoints, invalid ports, malformed UTF-8 and unqualified replies do not. Two permanent regressions failed before the fix. All 88 selected tests pass (50 Wi-Fi, 37 USB and one audio-USB test), including a creator-state regression, all 256 repeated-byte reply cases and port boundaries. The maintained classifier has 100% line coverage; all 28 measured functions in affected policy/reply/creation-test files have complexity <=9 (project Rust syn 2.0.119). At most two validated endpoint strings, each bounded by the 253-byte address limit, are retained within the call; native ownership transitions remain unchanged.

## T1555 — disable busy daemon controls

Service start/stop, system setup and desktop autostart now share the existing busy admission condition. Their visible/accessibility disabled state matches the action dispatcher while a save or another action is pending; unavailable platform capabilities remain disabled independently. The accessibility regression failed before the fix and now covers idle/action/save states with both running and stopped service status. All 19 selected GUI control/overview tests pass. All three maintained rendering functions have 81.82–100% line coverage; all 184 measured functions in the main/test files have complexity <=9 (project Rust syn 2.0.119). The change adds no retained state or native operation. Fixtures use private settings and settle their save workers; no running desktop/tablet session is changed.

## T1553 — validate media preferences before saving

Full configuration validation includes camera, microphone and speaker options. Invalid fingerprints, serials and conflicting selectors cannot be published; the GUI disables Save and displays the resource plus nested validation reason. The T543 fixture now checks display timing independently while requiring the full configuration to reject invalid camera FPS. Media sharing still starts only through explicit controls.

Three model regressions and the final visual/disabled-control regression failed without media validation. All 481 common-library tests and 17 selected GUI save/media tests pass. Boundary coverage includes fingerprint lengths 0–65, nonhex values, serial capacity/control boundaries, conflicting selectors and preservation of the previously saved bytes. The two maintained production functions have 83.33–100% line coverage; all 220 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119). Validation borrows settings and performs the existing bounded target parsing before opening publication resources; GUI formatting owns a short-lived error string. No media backend is started by validation or Save.

## T1650 — bind audio requests to their resource

Additional finding from this pass: a saved speaker profile could request microphone direction. Configuration validation now binds each audio field to its fixed resource. GUI entry points pass the owning resource explicitly, derive labels/capabilities from it and reject mismatched requests before consuming or creating a backend. A queued replacement retains both its validated request and original resource direction until the original native fence settles.

Both saved-configuration and workspace-start regressions failed before the fix. All 105 selected common-library tests and 47 selected GUI tests pass, including invalid targets, rejected starts, unchanged original ownership and queued replacement identity. All 11 maintained production functions have 91.67–100% line coverage; all 335 measured functions in affected files have complexity <=9 (project Rust syn 2.0.119). The change adds one bounded direction value to the existing single pending request; cancellation, failure and admission release that request through the original paths. Native cleanup ownership is unchanged. Native Windows acceptance remains tracked in T1271.

## T1547 — document retained display slots

SECURITY.md now matches the Linux monitor's implemented ownership: the serial and port slot remain reserved until original route creation/cleanup, device work and session teardown settle. Offline cleanup does not permit reuse; revoked tokens independently reject late connections. Verified against host/src/monitor/routes.rs::settle, retirement.rs::retire/retired and the existing architecture contract. Documentation-only change; no runtime code or tests changed.

## T1563 — retire ended Android workers promptly

The parent retains one generation-scoped terminal signal, including events received after Stop, and requests EXIT when it arrives instead of always consuming the grace period. Stale generations and nonterminal events cannot accelerate exit. Original process death, control-call completion and unbind remain required before retirement; failed cleanup still forces process exit rather than claiming native settlement. The mailbox accepts the already-decoded immutable message, preserving one encode/decode per boundary event and its existing count/byte bounds.

Both new regression methods failed on API 27 and 34 before the fix (four failing cases). All 76 selected hosted Android cases pass afterward, including original blocked START/STOP/EXIT, binding-loss and failed-unbind regressions. The 39 measured production methods/callbacks across the worker class and changed mailbox admission have 85.71–100% line coverage (JaCoCo 0.8.15); all 283 measured functions in affected source/test files have complexity <=9 (Kotlin PSI 1.9.20). The maintained admission/event/exit methods each have 100% line coverage.

The same hosted default-grace fixture measured Stop-to-retired at 512.57/511.52 ms before and 5.94/6.14 ms after on API 27/34. These are two samples per revision in simulated Binder environments, not native replacement-to-connected performance. T1651 retains physical-device timing and acceptance; no installed APK or live session was changed. The new deferred owns no native resource, retains one completion bit per original worker, and does not release existing binding/control ownership.

## T1559 — reuse the Video frame snapshot

Basic GPU and advanced pipe controls now borrow the frame's existing status snapshot. They neither lock the shared status during rendering nor clone its catalog again. A poisoned status producer follows the existing frame-level default recovery instead of panicking only on Video. The permanent full-window regression failed before the fix, then passed through both basic and expanded controls without clearing the poison. All 36 selected GUI tests pass. All three maintained production functions have 82.35–100% line coverage; all 180 measured functions in the main/test files have complexity <=9 (project Rust syn 2.0.119). The snapshot borrow ends with synchronous rendering and creates no new retained/native resource. Native Windows GUI acceptance remains tracked in T1271.

## T1558 — bound initial X11 placement

Initial placement stops after 25 unmapped-window observations and reuses one read-only X11 connection across those observations. Success, query failure, exhaustion and owner destruction close that connection; the application window remains borrowed from eframe. The existing 20 ms repaint request is limited to pending attempts.

The injected hidden-window regression observed 1000 queries before the fix and exactly 25 afterward. All ten selected tests pass, including all success/error completion positions within the 25-attempt budget and the existing private-Xvfb eframe checks at two scales with and without Metacity. A server-side pixmap proves the same connection survives pending observations and is destroyed on exhaustion while the application window remains valid. All five maintained production functions have 95–100% line coverage; all 24 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119). Native tests create their own display server; the user's display and running sessions are untouched.

## T1574 — passive GPU discovery and bounded label cache

The Linux poll owner retains one cache of current GPU identities/metadata and labels. An unchanged set reuses labels; changed nodes, driver/PCI metadata, disappearance or an unavailable inventory invalidate them. Permission evidence refreshes on every scan without opening render nodes, using effective credentials and ACL checks through [faccessat with AT_EACCESS](https://man7.org/linux/man-pages/man2/access.2.html). Opening/encoding remain separate native admission; the shared contract and GUI wording now say permissions, not verified device access. One-shot host discovery uses the same implementation.

Both permanent regressions failed before the fix: unchanged scans called the database loader three times, and an inotify watch observed a render-node open. All ten GPU tests and six GUI Linux status/GPU tests pass afterward, including node retargeting, changed metadata, removal/reappearance, denied/missing/NUL paths and oversized metadata/device sets. All 12 maintained production functions have 100% line coverage; all 47 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119).

For the same 30 immediate scans against an owned fixture and a 1,703,962-byte PCI database, strace recorded 30 database-read opens and 30 render-node read/write opens before; afterward it recorded one database-read open, zero render-node opens and 30 permission checks. Single traced debug-test timings were 29,458 µs before and 11,770 µs after; these are fixture observations, not native GUI power/latency claims. T1652 retains discrete-GPU runtime-suspend and open/closed GUI acceptance. No installed process was restarted.

The cache owns at most 256 current records; aliases are limited to 200 bytes, cached uevent reads to 4096 bytes and displayed product/driver names to 256/128 characters. The at-most-4 MiB PCI database is temporary and released after rebuilding labels. A rebuild temporarily retains old and new bounded records; published device snapshots own their derived copies. Removal/rejection replaces or clears cached records. Files and permission-path borrows settle synchronously; no render descriptor or native worker is retained.

## T1653 — reject obsolete empty-success package replies

Additional compatibility removal: current [Android package-path reporting](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/core/java/com/android/server/pm/PackageManagerShellCommand.java) returns exit 1 for absence and exit 0 with paths for presence (observed source blob aa235c2258ac152037b0bc33edf860de20624a7a). Empty success now remains unconfirmed. The old exit-0/empty fixture was retargeted to that contract; current absence, split-package presence, stderr and invalid-output rules remain covered.

Both new regressions failed before the fix. All 47 selected host-library inventory/USB tests and three Linux monitor package-probe tests pass. The permanent matrix covers every exit code 0–255, empty/whitespace output, invalid UTF-8, partial paths and stderr. The maintained parser has 100% line coverage; all 23 functions measured in affected parser/test files have complexity <=9 (project Rust syn 2.0.119). Parsing borrows the caller-owned bounded command output within the call, adds no retained resource and leaves original command/route retirement unchanged.

## Broad checkpoint through T1574

At implementation commit 33810de2, all 490 common-library tests and all 249 GUI tests passed. This includes private-display and private-process fixtures; it does not imply physical tablet or Windows native acceptance. Later increments retain their own scoped validation below.

## T1654 — keep portable GUI tests independent of Linux fixtures

Windows test compilation failed because camera tuning fixtures called a rendered-text helper inside the Linux-only root test module. The helper now has one platform-neutral test-module owner; every explicit caller uses that owner, without retaining the old exported path or gating portable tests out. The new T1654 camera-label regression was present during the failing Windows compile and remains in the ordinary test suite.

Windows GNU checking of common and GUI tests now passes, and all 58 selected Linux camera/audio/About tests pass. The existing Windows CI checks compile the portable regression and run the workspace tests on Windows; no remote job was started here. All 255 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119). This changes test infrastructure only: the recursive read-only helper borrows the frame's finite shape tree and appends owned labels to the caller's vector; no native resource or production behavior changes. Cross-compilation does not establish native Windows runtime acceptance.

## T1535 — explicit audio backend capability

Shared platform capabilities and diagnostics now include Audio. Linux reports an implemented backend with runtime readiness unverified; Windows reports unsupported. Microphone/speaker sharing remains unavailable on Windows, as the current platform guide now states. All current capability fixtures were updated together.

The diagnostic regression failed before adding the capability/report line. All four diagnostic integration tests, three platform/CLI tests and one GUI diagnostic test pass. Common and GUI tests also pass Windows GNU checking after T1654 repaired the independent test-helper defect; native Windows execution remains T1271. Both maintained production functions have 100% line coverage; all 32 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119). This adds a scalar capability and one bounded diagnostic string to existing call-scoped output; it starts no backend and introduces no native resource lifetime.

## T1592 — document current settings and CLI controls

Current guides now document the three default-enabled host activity pause settings, their daemon-restart boundary and their separation from auxiliary-media power constraints; the opt-in Windows direct-input table and finger-mouse permission; camera sensor CLI flags and exposure units; and audio processing, buffer and background flags. CLI startup flags are distinguished from saved preferences and GUI live adjustment. Defaults and limits were checked against the independent configuration/CLI contracts, host activity policy and display-work consumers, and Windows input preferences. This is documentation-only; no production behavior or resource lifetime changes and no artificial regression test were added.

## T1520 — explain pairing denial and saved-trust capacity

Denied pairing now reports “Pairing denied on tablet”; full remembered trust reports its configured limit and suggests forgetting a computer or pairing without remembering. No rejection, admission or cleanup policy changes. Three permanent regressions failed with the old generic message; all 44 selected setup/trust/diagnostic tests pass afterward. Boundary cases cover capacities 1, 64 and 1024, both denial preferences, unchanged saved records, cancelled authenticated authority, and temporary approval at capacity.

Both maintained production methods have 100% executable-line coverage (JaCoCo 0.8.15); all 112 measured functions in affected source/test files have complexity <=9 (project Kotlin PSI 1.9.20). The existing setup owner still owns the backend and trust store through their original retirement; denial performs no authentication or I/O admission, and failed remembered authentication cancels its ticket before propagating the diagnostic. The bounded error strings retain no peer list, native object or callback. Native device sessions were not touched.

## Installable local checkpoint — 43d0af25

A local Linux release bundle and designated-key Android APK are staged under `/home/netto/.cache/blent-checkpoint-20261121/artifacts/`. The archive is `blent-43d0af25-linux-local.tar.gz` (SHA-256 `81498fb760e75502f01c0505a63fec98112440aa1758538bf18b600fbf91e997`). This is a local build, not a portable AppImage or a published release. All seven staged ELF files resolve their dependencies on this machine; the CLI reports version 1.2.3. Android release signing, package identity and lint checks pass. No installed desktop file, service or tablet application was changed.

The host-library checkpoint passed 1,318 tests with three preexisting opt-in timing benchmarks ignored (T407, T416, T784). Android's full debug suite passed 2,933 tests before T1520; the subsequent T1520 increment passed all 44 selected setup/trust/diagnostic tests and rebuilt the verified signed APK. Earlier common/GUI and subsequent scoped/cross-compilation checks remain recorded above. These build/test results do not replace native sharing acceptance. Source code in the staged desktop build is unchanged from 92086ba2; the checkpoint includes documentation through T1592 and Android through T1520. Local missing libusb/PipeWire development files were extracted into a private cache; the existing pinned libevdi was borrowed read-only for linking. The installed application's dependencies were not modified.

## T1543 — release retired attachment metadata

The monitor now removes its attachment-map entry when it closes the original display ticket for retirement. This covers failed preparation and panicked/cancelled mutation completion as well as ordinary removal. Native session, route and mutation owners remain in the original retirement job; their completion still gates slot reuse. Retained watch metadata no longer grows with distinct failed serials.

Both permanent regressions failed before the fix. All 81 monitor tests pass afterward, including 32 sequential rejected identities, panic/cancellation, closed metadata watch senders, and the existing original-native-exit and route-retirement regressions. The maintained retirement method has 100% executable-line coverage; all eight measured functions in the changed production/test files have complexity <=9 (project Rust syn 2.0.119). Metadata release is synchronous at logical retirement; it does not substitute for native quiescence. The tests use owned synthetic objects and private subprocess fixtures, with no real tablet access.

## T1537 — consistent display-character validation

ADB serials, workspace tablet names and observed route keys/profile identities now use the existing shared USB-name character policy. Controls and explicit direction marks/embeddings/isolates are rejected; ordinary international text remains accepted. Identifiers are rejected rather than rewritten, preserving exact ownership keys. Existing byte limits and the empty workspace-name fallback remain unchanged.

Three permanent regressions failed before the fix. All 490 common-library tests and ten selected integration tests pass afterward. Coverage includes the twelve forbidden direction controls, all 65 C0/C1 controls at text boundaries, byte limits around 1024, empty values and Arabic/Hebrew/Japanese text. Each of the three maintained production functions has 100% executable-line coverage. All 51 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119), and common tests pass Windows GNU compilation; no native Windows rendering claim is made.

The validators borrow bounded caller-owned text for the duration of the call. Successful workspace/route construction owns the admitted strings; rejection drops the moved inputs, with no external effect, callback, native handle or retained queue. This does not establish the visual impact of previously accepted strings on a physical GUI.

## T1655 — back off retained Wi-Fi endpoint cleanup

The Wi-Fi cleanup owner now delays failed/unsettled observations by 100, 200, 400, 800, 1600 and 3200 ms, then caps the delay at five seconds. The shared private cadence stores one duration and does not own commands or alter settlement state. A new cleanup owner starts at 100 ms; successful immediate cleanup still returns without sleeping. Unknown creators continue waiting for their endpoint effect, and unacknowledged removals are never reissued. Inventory remains the only observation before any endpoint-specific mutation.

Three permanent virtual-time regressions failed at the second delay before the fix (100 ms observed, 200 ms required). All 54 selected Wi-Fi/cadence tests pass afterward, including original creator/removal effects, malformed inventories, cancellation, endpoint exclusion and graceful exit. The cadence stays capped across 4,096 further retries and independent owners start fresh. All three maintained production functions have 100% executable-line coverage from this run's own profile; all 15 measured functions in affected production/test files have complexity <=9 (project Rust syn 2.0.119).

The attempt retains its original admission, address reservation, command fences and cleanup sponsor through every delay. A timer cannot settle an unknown obligation; eventual cleanup still requires native completion and valid confirming observations. No queue, detached callback or new retained native resource is introduced. Display/media adoption remains T1656/T1657 and native CPU/process-rate acceptance remains T1658; T1542 is not complete.

## T1659 — validate identities at the display-route boundary

Follow-up to T1537: display-route admission and retargeting now call the authoritative shared ADB identity validator directly. The duplicate control-only validator was removed. Both permanent rejection regressions failed before the fix; all 63 USB tests pass afterward, including unchanged original serial/control identity after rejection, supported international/network forms, size boundaries and existing route/native settlement checks.

Both maintained production methods have 100% executable-line coverage using only fresh profiles from this run. All 15 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119). Validation borrows the candidate serial before allocating or changing the bounded owned serial; rejection cannot mutate the shared control or endpoint ownership. Existing route workers, native leases and fences are unchanged. No real device commands were issued.

## Updated installable local checkpoint — e657e8c4

The current Linux release bundle is staged at `/home/netto/.cache/blent-checkpoint-20261121/artifacts/blent-e657e8c4/`, including the verified signed `blent.apk` and `CHECKPOINT.json` hashes. It adds T1543, T1537, T1655 and T1659 to the earlier checkpoint. The release build includes native audio and accessory transport; all seven staged ELF files resolve their dependencies locally and the CLI version check passes. The APK was verified again after staging; Android source is unchanged since its signed 43d0af25 build. Windows GNU host-library compilation with accessory transport also passes using the existing isolated cross-compiler. This is a local Linux build, not a portable AppImage; native Windows execution and physical sharing acceptance are not implied.

The original 112 open items now have 27 resolved. Five additional findings were resolved, plus the T1655 cleanup sub-outcome; its parent T1542 remains open. The current working ledger has 87 open rows (10 medium, 77 low), 16 blocked rows and eight declined rows. Two new open cleanup sub-outcomes explain why the open-row count did not fall by exactly 27. No installed desktop/tablet files or running processes were changed; the temporary signing-properties copy was removed after signed-APK verification. Reinstallation/reload still requires an idle window outside the owner's active call.

## Installed and reloaded — e657e8c4

Owner instruction “reinstall and reload” authorized the interruption. The desktop service and original USB broker stopped cleanly; the owned display reverse mappings were gone before installation. The current tested binaries were installed through the existing extracted-AppImage registration path, retaining the bundled FFmpeg/ADB tools and using the package's runtime-path helper. A rollback copy and registration/configuration snapshot are retained at `/home/netto/.cache/blent-reload-20261121-e657e8c4.yoVFoW`.

The connected RugKing Pad 2 Pro (8002RH1010011900) accepted the signed APK with `adb install -r`, preserving app data. Its installed APK hash matches the verified artifact exactly. The tablet application, desktop daemon, USB broker and GUI were restarted; running desktop executable hashes match the staged update. The tablet reconnected over USB ADB and fresh render acknowledgements were observed. Reopening the GUI returned successfully and retained the same sole GUI process. Parsed desktop configuration is unchanged. Camera/audio capture, unplug/replug performance and Windows native acceptance were not performed; their remaining ledger obligations are not closed by this deployment.


## T1656 — back off retained display mapping cleanup

Display cleanup now uses the shared capped retry cadence and checks validated device inventory before querying the selected tablet. Absence, malformed/oversized inventories and failed commands retain the original creator states and route reservation. Inventory and physical-identity validation use one serial snapshot even if the controller retargets during the query. Native command fences and original route settlement remain unchanged.

All three original regressions failed before the fix; all 68 USB tests and 81 monitor tests pass afterward. Two additional boundary/interleaving tests cover concurrent retargeting and rejection of a serial-only owner changing identity. Existing private fixtures now explicitly model device return before cleanup; they preserve assertions for foreign mappings, delayed creators and native retirement. All five maintained production functions have 100% executable-line coverage; all 129 measured functions have complexity <=9 (project Rust syn 2.0.119). Profiles are restricted to the final USB run.

The existing bounded worker remains the cleanup sponsor across every delay, with one bounded inventory string and one serial snapshot per observation. Absence never releases native leases or unknown effects. Conditional eventual cleanup still requires the original native commands to complete and the original tablet to become observable. No real Blent process or tablet was touched. Temporary media adoption remains T1657; physical acceptance remains T1658.

T1656 integration follow-up: the original process-exit fixture now supplies its connected tablet inventory. The portable native-command test and both process-exit tests pass, including normal/error/cancel/panic drains. All eight fixture functions measure <=9 complexity.


## T1521 — stop remembered retries when the accessory disappears

Before a remembered retry reads settings or admits a connection, Android now checks for a currently supported accessory. An absent, unsupported or unavailable snapshot ends that retry chain; a fresh attach event or manual Connect remains available. Presence observation does not consume the automatic attach opportunity. Attach and retry observations share one native snapshot slot; a cutoff or detach during a snapshot invalidates its result.

The Activity regression failed on both API 27 and API 34 before the fix. All 41 selected connection, attachment and remembered-retry tests pass afterward, including snapshot exclusion, cancellation, user cutoff, unsupported metadata, retained original retirement and attach-event consumption. All six maintained production methods and three separately measured callbacks have 100% executable-line coverage (JaCoCo 0.8.15). All 70 measured functions in affected source/test files have complexity <=9 (project Kotlin PSI 1.9.20).

The application still owns the receiver and the single native snapshot slot. The snapshot remains sponsored through cancellation until the original call returns; only then is its mutex released. Each observation retains one epoch identity and one bounded accessory snapshot (capacity 32), introduces no native device opening and cannot grant media access. Activity retry state clears when admission returns no attempt. Tests use Robolectric and owned fake workers; physical Android USB acceptance was not performed.


## T1657 — back off retained camera/audio mapping cleanup

Temporary media mappings now share the capped cleanup cadence and validate inventory before device-specific queries. Missing, malformed, oversized, non-UTF-8 and failed inventories retain original endpoint locks and creator/removal obligations. Inventory reads use the original native child/collector fence; caller loss does not release endpoints while the query remains live. Valid nonready entries remain present. No original command, mutation or settlement contract changed.

Both original regressions failed before the fix. All 43 mapping/adapter tests pass, including real private subprocesses, held inventory commands, late creators/removals, malformed replies, exact 64 KiB bounds and endpoint reuse only after retirement. All 1,330 host-library tests pass; three existing opt-in benchmarks remain ignored (T407/T416/T784). All three isolated native audio tests pass, covering microphone PCM/underflow/retirement, stereo speakers and fixture endpoint identity, plus two Python fixture tests. The isolated PipeWire servers use private runtime directories and no physical devices. Existing fixtures now supply their actual simulated inventory; inventory parser tests moved unchanged into an explicit module so the integration target can include the production parser.

All three maintained production functions have 100% executable-line coverage from the final mapping-test run. All 142 measured Rust/Python functions in affected production/test files have complexity <=9 (project Rust syn 2.0.119 / Python AST). The retained worker owns its original control, endpoints and bounded command proofs across every delay; inventory storage is call-scoped and capped at 64 KiB. Eventual cleanup remains conditional on native settlement and a valid observation of the original tablet. Physical unplug/replug rates and CPU remain T1658, so integration parent T1542 remains unresolved pending that acceptance.


## T1602 — ignore cancelled resource admission notices

A queued Admitted notice for the same original lease now becomes a no-op when that lease is no longer current. It neither admits a connector nor fails the USB link. Unknown/stale identities and duplicate current admissions still reject; Closing/Retired notices retain their existing exact cleanup rules.

The permanent regression failed before the fix and all 54 selected bridge/session/link tests pass afterward. It checks cancellation both before and after Closing delivery, no camera connector polling, exact connector/socket destruction, and preservation of admitted display streams through camera retirement. The maintained state method has 96.97% executable-line coverage. All 35 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119). This changes no allocation or native ownership: the original entry remains owned until its closing/retirement path settles. A separate confirmed connector-poll-after-cutoff issue is tracked as T1660.


## T1660 — fence local bridge polling at authoritative cutoff

Every local bridge turn now checks the authoritative session lease before polling its connector or pumps. A cancelled lease synchronously retires the original local owner and reports progress, even when its Closing notice is still queued. Sibling resources retain their independent turns and ownership.

The permanent regression failed before the fix (one connector poll after cutoff instead of zero). All 56 selected bridge/session/link tests pass afterward. Eight bounded interleavings cover unstarted camera/microphone/speaker connectors, pending connectors and established streams. They verify no post-cutoff I/O, exact connector/socket destruction, unchanged sibling display streams and reuse only after the original peer/native retirement. The maintained poll method has 100% executable-line coverage; all 38 measured functions in affected files have complexity <=9 (project Rust syn 2.0.119).

No new state or allocation is introduced. The existing connector contract requires synchronous destruction of all native I/O; process-backed adapters remain excluded from this boundary. Only after dropping the original connector/pumps does retirement publish native completion. Peer retirement is still independently required before slot reuse. This is deterministic mock-stream evidence, not physical USB timing acceptance.


## T1491 — preserve the live-trial window across sample eviction

Each encoder trial now retains one epoch-bound first-fresh-ACK timestamp separately from the existing 256-sample buffer. Warmup exclusion follows the original first three ACK ordinals instead of skipping three more samples after each eviction. The retained window still needs at least 12 post-warmup samples, two seconds of observations and 90% delivery; the six-second deadline remains unchanged. Missing the original first ACK fails closed, and an encoder epoch change requires its own startup evidence.

The permanent virtual-time regression failed before the fix. All 99 selected selection/latency tests pass afterward. The recovery case simulates 60 fps with 30 early lost frames, both on a new epoch and after 20 preceding ACKs; it recovers after more than 256 new ACKs while retaining the original startup timestamp and the 256-sample bound. Additional tests cover absent/evicted first ACKs, epoch replacement and ordinal overflow boundaries. The three maintained production functions have 97.5%, 96.15% and 100% executable-line coverage; all 22 measured functions have complexity <=9 (project Rust syn 2.0.119).

The trial future owns one optional epoch/timestamp pair, released on success, cancellation or timeout. It retains no encoder Arc, native object or borrowed sample. Existing generation ownership and bounded snapshot allocations are unchanged. This demonstrates trial-policy recovery, not an improvement measured on physical displays.


## T1522 — propagate the remaining TLS setup budget

The independent setup contract now passes the remaining native/approval budget through the Android adapter to the socket backend. TLS handshake reads use that budget instead of a fixed ten seconds; authenticated-preface reads refresh against one absolute deadline, including a final expiry check before admission. Successful TLS still restores the caller's original stream timeout. Current callers and interop fixtures use the explicit budget contract; no old-signature wrapper remains.

The new contract was wired before fixing the native implementation. The permanent socket regression then failed with timeout writes `[321, 10000, 321]` despite a 25000 ms setup budget. All 88 selected Android tests and 14 maintained Rust/Android TLS interoperability tests pass after the fix. Coverage includes native/approval budget minimums, 1/25000/60000 ms limits, expired and invalid budgets, truncated prefaces, failed/late TLS, concurrent close, certificate/ALPN rejection and API 27/34 paths. All nine maintained production methods/constructors and two measured callbacks have 100% executable-line coverage (JaCoCo 0.8.15); all 668 measured functions in affected Kotlin source/test files have complexity <=9 (project Kotlin PSI 1.9.20).

The remaining-budget callback borrows the original socket setup owner only during its synchronous native call. The ten-byte preface buffer and read offset remain call-scoped and range-checked. Socket handoff, failure close, separately sponsored closer and original worker-retirement obligations are unchanged; a deadline still cannot authorize native reuse. The interop tests use owned loopback peers and private subprocesses. No installed app, physical tablet or live sharing session was changed.


## T1564 — await framed decoder replacement retirement

A repeated VP9/AV1 configuration now observes the previous codec's native retirement for up to 500 ms before admitting its replacement. The observation copies the original owner under the receiver monitor and joins outside that monitor. Expiry retains the owner and leaves replacement admission closed; setup rechecks the stream generation and surface identity after waiting.

The permanent second-configuration regression failed before the fix. All 94 selected Android decoder tests pass afterward, covering both framed codecs, API 27/34, a held native borrower, wait expiry, invalid observation bounds, receiver Stop and surface loss during retirement, startup failures and native release failure. The two maintained production methods and the setup validity callback have 100% executable-line coverage (JaCoCo 0.8.15). All 180 measured functions in affected source/test files have complexity <=9 (project Kotlin PSI 1.9.20).

The existing CodecLifetime remains the authoritative cleanup sponsor. Retirement closes borrowing, waits for admitted operations, then stops/releases the original native codec; the global gate prevents another session from replacing an unsettled owner. The observer retains one call-scoped owner reference and creates no native resource. Neither timeout nor logical Stop authorizes reuse. This is deterministic Robolectric evidence, not physical decoder timing acceptance.


## T1566 — align decoder eligibility and reserve family detail capacity

Shell and negotiation inventories now share case-insensitive MIME matching and reject metadata requiring secure or tunneled playback. Capability lookup uses the exact advertised spelling. Negotiation keeps at most four unique eligible detail rows per family in platform order, retaining the existing 16-row total bound and preventing early families from excluding later families. The existing duplicate/invalid-name regression now checks the stricter four-row family budget.

With verified restricted-capability fixtures, all six original regression cases failed before the production fix on API 27/34. All 72 selected decoder inventory, negotiation and version tests pass afterward. They cover both restricted features, mixed-case MIME, 80 decoders across four families, invalid/duplicate identities, failed capability queries and bounds. Named selection already rejects restricted codecs through Android's regular inventory; its new safety regression passes without a production change. No physical device advertising restricted codecs in its regular inventory was observed.

All six maintained production functions and one measured callback have 100% executable-line coverage (JaCoCo 0.8.15). All 111 measured functions in affected production/test files have complexity <=9 (project Kotlin PSI 1.9.20). The API 27 fixture explicitly fills and verifies required-feature metadata because Robolectric 4.17's builder omits those flags below API 29, and framework capability copies reconstruct them from the format.

The metadata query still owns one request-scoped snapshot; the shared policy only borrows metadata and opens no native decoder. Each family retains at most four accepted names/JSON rows, with existing bounded profile/name/message limits. No native handoff or cleanup boundary changes. This is inventory-policy and fixture evidence, not a physical decoding or performance claim.


## T1565 — exclude idle time from decoder stall accounting

The watchdog starts its no-output observation when the first picture is submitted after output or restart. An idle decoder has no outstanding output obligation. The four-picture minimum, strict greater-than-1.5-second boundary, two-stall hint fallback and sustained-recovery requirements remain unchanged. This resolves the conflicting sparse-window fixture by timing its unanswered pictures from admission, while retaining its stall assertion. Native watchdog fixtures now admit a picture before advancing their clock.

Both new regressions failed before the fix. All 58 selected Android tests pass afterward, including 192 seeded idle/resume bursts, negative and wrapping monotonic timestamps, exact expiry boundaries, genuine unanswered work, repeated stalls, sustained/insufficient recovery, stop/start and retired callbacks. All five measured watchdog methods/constructors have 100% executable-line coverage (JaCoCo 0.8.15), including the maintained constructor and queued method. All 105 measured functions in affected production/test files have complexity <=9 (project Kotlin PSI 1.9.20).

One watchdog remains owned by each DecoderSession. The change adds one scalar timestamp, guarded by the existing synchronized methods; it retains no frame, callback or native reference. Output/restart clears the pending count, and the next admission replaces the timestamp. Timestamp differences use the existing monotonic-clock contract, including ordinary signed wrap. Native decoder retirement and exact callback identity checks are unchanged. Tests use virtual clocks and Robolectric API 27/34; physical idle/resume timing was not measured.


## T1516 — diagnose authorized tablets without the app

Without a live daemon session report, doctor now chooses from the observed authorized devices using the existing USB-preferred selection policy, after physical-identity deduplication. It no longer filters candidates through application presence. The subsequent package check can therefore report the missing application and the exact tablet installation command. Live daemon sessions and their actual port assignments remain authoritative.

The permanent private-ADB regression failed before the fix because zero candidates were returned. All 50 doctor tests pass afterward, including active native/ADB sessions, unavailable inventories, device classification and actual forwarding ports. The existing no-daemon fixture now expects the first USB candidate and asserts that package filtering was not called; its live-session assertions remain unchanged. The maintained diagnostic_sessions function has 100% executable-line coverage. All 99 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119).

Selection borrows the existing device snapshot and owns only the returned diagnostic record. Original command observation/retirement, device-identity evidence and output bounds are unchanged. Tests use a private executable and temporary files; no real ADB endpoint, tablet app or sharing session was changed.


## T1570 — use whole-megabit tablet bitrate stops

The tablet bitrate slider now has one stop per whole Mbps from 1 to 60, including the 20 Mbps default. Its label derives from the same rounded kilobit value submitted by Apply, preserving exact existing fractional settings such as 25005 kbit/s until the user changes them. The default, capacity limits and persistence contract are unchanged.

All four regression cases failed before the fix on API 27/34. All 56 selected settings UI tests pass afterward, including lower/upper stops, adjacent whole values, return to default, fractional labels and submitted values. The maintained BitrateControls function has 100% executable-line coverage (JaCoCo 0.8.15); all 102 measured functions in affected source/test files have complexity <=9 (project Kotlin PSI 1.9.20).

Formatting owns only call-scoped managed numeric/string values within the existing composition. Its number comes from the same Int kilobit conversion already used for submission, so decimal storage is bounded; it introduces no native resource, asynchronous callback or retained owner. This fixes the grid/label mismatch. Independent stream-setting synchronization remains T1569.


## T1614 — terminate concurrent USB failures without exit handlers

The USB stream helper enters a fatal-error mode before creating its reader thread. Errors still print their diagnostic, then terminate all process threads with _Exit(2), avoiding concurrent exit-handler execution. Single-threaded discovery/probe phases retain ordinary diagnostic exit handling. Normal stream completion continues to use _Exit(0).

The permanent regression starts both native failure paths behind a pthread barrier and registers an exit-handler marker. Before the fix, both error diagnostics and that marker appeared. Afterward, all 39 accessory/probe tests pass, including 16 simultaneous-failure executions with exit status 2 and no handler marker. The fixtures compile with GCC 13.3.0, warnings as errors, ASan and UBSan. They use fake USB operations and real private process/thread lifetimes, never physical devices.

The three maintained production functions have 100%, 85.71% and 100% executable-line coverage (GCC/gcov 13.3.0). The fatal _Exit statement is exercised by the subprocess regression but cannot flush its own coverage counters; this remains visible as the one uncovered line. All 49 measured functions in affected production/test files have complexity <=9 (project tree-sitter 0.25.2, C grammar 0.24.2, Python AST 3.12).

The process owns its USB state, lock and both native directions. One scalar mode is set by the main thread before pthread creation and remains immutable afterward. No thread frees shared libusb state while another transfer can access it; process exit ends both directions, and the existing supervisor retains admission until original process retirement is observed. The fixture's barrier and callbacks are process-scoped. No physical disconnect acceptance or unrestricted native safety claim is made.


## Android integration checkpoint

The complete Android unit suite passes all 2970 cases, with zero failures, errors or skips, after the decoder retirement, inventory, watchdog, TLS and bitrate changes. This is Robolectric/JVM evidence. T1486 remains open: the earlier joined-speaker failure was not reproduced or explained, and another successful full run does not establish its cause or resolution. Installed host/tablet applications were not changed during these source fixes.


## T1515 — report actual service and desktop autostart observations

Doctor now reads a bounded systemctl state even when is-enabled exits nonzero. Disabled service state retains its enable hint; static, masked and enabled-runtime are reported by name instead of being mislabeled disabled. Missing command evidence is explicit. A valid desktop login entry uses the shared XDG parser and reports enabled without requiring a systemd unit. Unreadable desktop entries remain unavailable rather than silently disabled.

All four command-path regressions failed before the fix. All 59 doctor tests and both shared autostart tests pass afterward. Six private subprocess fixtures exercise the real observation path with isolated PATH/configuration; additional cases cover every invalid internal byte, the 64-character state limit, malformed/hidden entries, invalid UTF-8, nonregular files, missing paths, and exact 64 KiB desktop-file bounds. No live service or login setting was modified.

All eight maintained production functions meet the per-function executable-line requirement: state observation is 87.5%, desktop-file observation is 84%, and the other six are 100%. All 136 measured functions in affected production/test files have complexity <=9 (project Rust syn 2.0.119).

Native commands retain their existing bounded child/output ownership. The desktop observer owns one nonblocking-opened regular-file descriptor, reads at most 64 KiB plus one sentinel byte into call-scoped storage, validates size again after reading, and drops the descriptor on every return. This is an input-length bound; allocator capacity and metadata overhead are separate. The shared parser borrows the call-scoped text; no setting, process, callback or native handle survives the observation. Missing measurements for Windows remain outside this Linux-only doctor change.


## T1501 — bound and throttle malformed controller diagnostics

Each input listener owns one warning window shared by successive controllers. It emits at most one malformed-input warning per second and counts suppressed messages with saturating arithmetic. Parser errors and frames each use an escaped 256-byte excerpt plus an optional three-byte truncation marker; formatting stops at the bound without allocating the complete parser diagnostic. Valid input and controller retirement remain unchanged.

Both initial regressions failed: one unknown-type frame emitted 120221 log bytes, and a 32-frame burst emitted 32 warnings. All 153 input tests pass afterward, including controller replacement, exact time boundaries, a regressed clock, counter exhaustion, Unicode/control input and a formatter that ignores truncation errors. All six maintained production functions have 100% executable-line coverage; all 103 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119).

The listener owns only a timestamp, saturating counter and mutex; it retains no frame or parser error. The mutex is released before logging. Each admitted warning temporarily owns two strings with at most 259 bytes of content each, dropped after synchronous emission; allocator bookkeeping is separate. Rejection stores no text, and no native handle, task, queue or callback is introduced. Existing controller/native retirement ownership is unchanged. Tests use private log files and fake input sinks, without modifying installed sessions.


## Installed and reloaded — f54ed991

The owner authorized finishing T1501, reinstalling/reloading, then continuing the backlog. The Linux release build includes native audio and accessory transport. The Android release build, lint and designated-certificate/package verification pass; temporary signing properties were removed. Seven staged desktop files match their installed hashes, and the installed tablet APK matches the signed artifact. This is a local Linux build, not a new portable or published release. Three unused CLI helper warnings are recorded under T1661 for subsequent removal.

The original daemon, capture/encoder and USB broker retired before installation; owned reverse mappings were absent. The extracted-AppImage registration route preserved bundled FFmpeg/ADB, app data and the autostart preference. Rollback and private configuration snapshots are retained at `/home/netto/.cache/blent-reload-20261121-f54ed991.q8weqhdu`. Parsed desktop settings remain unchanged.

The RugKing Pad 2 Pro reconnected over USB ADB. Capture initialization completed after approximately 12 seconds; the early codec/mapping warnings recovered once the virtual output became available. Fresh render acknowledgements were observed (184 samples in the first five-second reporting interval). Running daemon, broker, GUI and capture-helper executable hashes match the update. Reopening the GUI retained the same sole GUI process. Camera/audio capture, physical unplug/replug recovery and Windows native acceptance were not performed. Source work continues without restarting this installed checkpoint again.


## T1662 — keep native Wi-Fi fixtures on the current reply contract

The private native ADB fixture now emits `connected to <requested endpoint>` on success. Its rejected `disconnected` reply remains unchanged. The existing T105 test failed before the correction and now passes its original saved-address replacement, Wi-Fi-off and exact command-log assertions. The T691 rejected-connect control also passes. No production contract or assertion was relaxed. The temporary fixture owns only its script, route/log files and the existing bounded command children; no installed ADB state is touched. The fixture function measures complexity 2 with the project Rust syn 2.0.119 checker.


## T1663 — expose connected inventory in the display retry fixture

The retained T144 native fixture now answers `adb devices` with its actual PRIMARY and EXTRA identities. Its original regression failed because the stricter cleanup correctly refused the fixture's empty, invalid inventory. It now passes unchanged lost-acknowledgement, retry-backoff, both-port, launch-count and original-retirement assertions. This is a fixture correction; production unknown-inventory retention is unchanged. Private route files and child processes retain their existing cleanup sponsor and original completion boundaries. The maintained fixture method remains within complexity 9 under the project Rust syn 2.0.119 checker.


## T1661 — remove obsolete tablet-selection implementations

Removed the unused CLI app-installed wrapper, batch package-filter implementation, combined selector and test-only duplicate Wi-Fi selector. Permanent T142/T250 regressions now use current incremental Discovery, package observation and tablet selection; T142's Wi-Fi outcome and T278's mDNS exclusion exercise the actual shared Wi-Fi selector in its own module. No old implementation or compatibility wrapper remains. All applicable app-presence, preferred-tablet, extra-slot, wireless-exclusion and exact probe-count assertions remain.

All 45 CLI tests and both shared Wi-Fi selection tests pass. The Linux release CLI build with native audio/accessory transport completes without warnings. All 120 measured functions in affected source/fixture files have complexity <=9 (project Rust syn 2.0.119). Retained production method bodies are unchanged; there are no new production methods needing coverage counters. The fixture's incremental discovery owner explicitly stops and awaits its original jobs before returning eligible values; native command fixtures remain private and bounded. Installed f54ed991 applications remain running unchanged.


## T1499 — preserve touch cancellation on the wire

Touch action 3 carries cancellation for Android system cancellation, late palm classification and replacement of an unfinished gesture. Direct input maps it to its existing Cancel phase, preserving the Windows adapter's cancelled-release flag. Linux uinput has no such flag: its adapter retires only that slot without synthetic coordinates. This Linux capability limit is explicit; an application can still interpret the native release normally. Ordinary touch-up remains action 1.

All three initial host regressions and six Android cancellation cases failed before the fix; two ordinary-release controls passed. All 156 host input tests and 98 selected Android tests pass afterward. An additional bounded test checks all 256 action bytes against the four defined actions. Existing invalid-action and palm regressions retain their oracles under the updated wire contract. All three maintained Rust functions and both maintained Kotlin methods have 100% executable-line coverage. All 297 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119 / Kotlin PSI 1.9.20).

Android's existing translator still owns at most ten contact slots; cancellation sends each terminal event before clearing its assignment, and a refused send retains existing synchronous connection retirement. Linux keeps the original native release debt through write failure and only permits fresh contact reuse after confirmed release. The action adapter is stateless over one byte. No new native resource or asynchronous owner is introduced. Physical Windows cancelled-touch acceptance remains with the existing native-input validation obligations. Installed host/tablet applications remain at f54ed991.


## T1669 — preserve active media when closing the window

A root close request now minimizes the window while camera, microphone or speakers retain active intent, queued starts or original retirement. It cancels window destruction and retains the original App/backend. The taskbar remains a recovery path even without a daemon tray; Linux tray/application activation restores the same window. Explicit Stop retains its original resource-retirement behavior, and an idle/terminal window closes normally. No new startup or periodic repaint is introduced by the close policy. Camera/audio instructions now describe this behavior.

The initial close regression failed before implementation. All 257 GUI unit tests, both process-exit tests and the startup/accessibility integration test pass. Seven close-specific tests cover camera/audio states, both audio directions, pending acknowledgements, replacement intent, original cleanup, ordinary frames and a real eframe window on private Xvfb. The native-window test closes, activates the original socket, verifies the same synthetic camera owner, then explicitly stops and observes exactly one destruction. The narrow-window regression caught a long hint pushing controls out of view; shortening the hint preserves its unchanged visibility assertions.

All seven maintained production functions meet per-function executable-line coverage: five are 100%, camera summary is 91.67%, resource rendering is 87.88%. All 306 measured functions in affected production/test files have complexity <=9 (project Rust syn 2.0.119). Native Windows GUI acceptance remains T1271. This is private-window and synthetic-backend evidence, not a physical camera acceptance run.

The App/panels remain the authoritative media owners. The policy only borrows their current intent/state and emits two frame-scoped viewport commands; it adds no retained state, allocation pool, native handle, callback or worker. Minimizing is deliberate retention while sharing is requested. Stop, normal idle exit and existing process-retirement sponsors still own cleanup; no timeout or visibility change settles a native resource. Installed applications remain at f54ed991 until another authorized update.


## T1664 — separate Microphone and Speakers tabs

The desktop now has Microphone and Speakers tabs, each rendering only its own settings and Start/Stop controls. Overview Configure actions route to the corresponding tab. One typed audio direction selects the existing panel and preferences; the direction participates in the tab's UI identity. Shared labels come from one function, and the old combined Audio tab is removed. Current audio instructions match the two tabs.

Both initial navigation/routing regressions failed before the change. All 259 GUI unit tests, both process-exit tests and the startup/accessibility test pass afterward. Existing per-direction Start/Stop/Save, paused-intent, wrong-direction rejection, draft-preservation and narrow-window Save tests remain; both tabs are included. All six maintained production functions have at least 80% executable-line coverage; exact counters are retained in t1664.json. All 290 measured functions in affected source/test files have complexity <=9 (project Rust syn 2.0.119). Windows native navigation/keyboard acceptance remains T1271.

The tab carries one immutable enum value and allocates no resource. The microphone and speaker panels remain the separate authoritative owners, including their admitted targets, pending starts and native retirement fences. Rendering borrows only the selected panel/preferences; navigation does not start, stop, retarget or duplicate media, and the existing common maintenance loop still services both panels. Source changes do not claim a new audio backend or NFC support.


## GUI-only installed update — 319097ff

The verified webcam close/reopen and Microphone/Speakers tab changes are installed in the existing AppDir. The GUI release build includes native audio and accessory transport. Its patched runtime dependencies resolve locally; atomic replacement retains an exact rollback at `/home/netto/.cache/blent-gui-update-20261121-319097ff.c984125s`. The camera/audio guides were updated alongside the executable. No GUI process was running before this update.

The new GUI is running and its executable hash matches the staged update. A second launch restores the same PID. The display daemon retains PID 3258545 and the host/tablet checkpoint f54ed991; neither was restarted for this GUI-only update. Camera and microphone were not started by this verification. The T1499 touch-wire update remains source-only and requires a later matching host/tablet deployment. The close/reopen behavior has the private native-window and synthetic-media regression evidence recorded under T1669; no physical camera capture acceptance is implied.

## T1670 / T1671 — measured camera filtering capacity

T1670 retains reproducible bundled-FFmpeg probes and three final synthetic sweeps (81 decoder observations); see [the report](2026-11-21-ffmpeg.md). Larger pipes did not produce a consistent meaningful gain, so pipe defaults remain unchanged. Two filter workers improved finite throughput wall time by about 37–38%; paced CPU/tail results vary and are reported without a live-camera claim.

T1671 adds one portable bounded `filter_threads` profile value, default 2, range 1–4, exposed in CLI and Advanced camera settings. It applies on camera restart; codec threads remain 1 and active-session image adjustments keep their original revision. Five runtime assertions failed before the change; six targeted tests pass afterward. Common camera tests: 40 passed; host camera tests: 165 passed; GUI tests: 260 unit plus 3 integration passed. Four maintained production methods have 100% line coverage, and all 79 examined Rust functions score ≤9. Single-picture/no-successor delivery passes at each worker count. [Verification](artifacts/2026-11-21-open-items/t1671.json).

The new scalar adds no host queue or retained frame. Filter threads and scratch memory remain owned by the original FFmpeg child through exact retirement; input dimensions and worker count are bounded. Native peak RSS and Windows GUI behavior remain unmeasured. This source change has not been installed; the running GUI/display/tablet versions recorded above remain unchanged.

## T1672 — exercise the real ADB inventory query in its integration harness

The harness imported the production inventory module but never called `query`, producing a dead-code warning. The new permanent regression uses the private ADB program for ready/offline, confirmed-empty, malformed, invalid-UTF-8, failed-exit and missing-program outcomes. No production code or physical ADB state changed. All 44 mapping-retirement integration tests pass with no compiler warnings; all examined harness functions have complexity ≤9. The original private server is still killed and waited before its temporary directory is released. [Evidence](artifacts/2026-11-21-open-items/t1672.json).

## T1675 — skip unavailable GPU capture inspection

GPU helper availability and the session failure latch are checked before DRM/desktop inspection. The duplicated two-stage entry was merged; existing success/failure tests use the actual entry with an injected connector view. The new counter regression failed before the fix (one inspection instead of zero), then passed for all absent/failed/admitted combinations. Rejected states dispatch no inspection; admitted state dispatches exactly one. No elapsed-time gain is claimed from this call-count observation.

All 255 capture-matching tests pass; two existing opt-in timing experiments remain ignored. The maintained `try_start` method has 100% executable-line coverage and the examined adapter/test functions all score ≤9. Native child ownership, monitor identity and one-way failure behavior are unchanged. [Evidence](artifacts/2026-11-21-open-items/t1675.json). This is source-only; active capture was not reloaded.

## T1673 — bound idle lease publication without delaying revocation

The worker still consumes every receipt/viewer notification and ticks its policy, but publishes only when its cadence changes or the 250 ms renewal interval is due. Rejection bypasses scheduling, and original Stop/viewer-epoch cleanup is unchanged. The scalar publication history advances only after successful IO, using that attempt’s original start time; native blocking must not postpone its next check. Post-wait admission and receipt-expiry checks remain intact.

The permanent private-file regression failed before the fix with 61 publication/clear attempts for 60 simulated frames in one second; it passes with 4. This measures adapter calls, not live CPU savings. Two existing T1115 publication-wait tests exposed completion-time stamping during implementation; neither safety oracle was weakened, and both pass with original-attempt stamping. All 28 shared idle tests and 258 capture-matching tests pass; two pre-existing opt-in timing experiments remain ignored. Six maintained production methods measure 93.55–100% executable-line coverage; all 70 examined functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1673.json).

The per-worker scheduler owns no grant, queue or native handle. Original file/inode/token/worker retirement remains the cleanup sponsor; expiry does not establish release. Installed sessions were not changed. T1620 still retains the separate encoder-probe attribution and combined acceptance obligations.

## T1677 — retarget the FFmpeg packaging fixture

The existing T563 prefix/job test still supplied an obsolete prebuilt-runtime tool entry. It failed when the current builder requested source-built FUSE inputs from its deliberately empty fixture. The test now mocks the current runtime-build and source-recipe-copy boundaries and asserts that the built runtime reaches `--runtime-file`; existing FFmpeg prefix/job/source assertions remain. All 16 FFmpeg bundle/identity Python tests pass. No production code changed for this fixture correction; no real runtime build/download is admitted. [Evidence](artifacts/2026-11-21-open-items/t1677.json).

## T1676 — fingerprint the packaged FFmpeg runtime

The tuning cache now hashes the native FFmpeg executable and declared codec files behind its launcher. Packaging emits a bounded [runtime declaration](../ffmpeg-runtime-identity.md); the consumer rehashes actual bytes. Changing the executable with unchanged launcher/version text invalidates the tuning stamp. Native ELF programs work without a declaration; opaque scripts or invalid declarations decline caching and retain fresh measurement. There is no old launcher-only identity fallback.

Both permanent regressions failed before implementation. Afterward, 32 cache Rust tests, 16 FFmpeg bundle/identity Python tests and 28 AppImage Python tests pass. All 12 maintained production functions have 89.47–100% executable-line coverage; all 81 measured production/test functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1676.json).

Hashing owns one descriptor and one 64 KiB buffer at a time, with explicit metadata/component/count bounds. No reference escapes and errors close resources. Tests use private files and wait for their original subprocess before cleanup. The declaration does not fence concurrent external package mutation or cover undeclared system libraries. This is source-only; no installed process or FFmpeg file changed.

## T1674 — retain fresh FFmpeg capability probes

Sixty private native H.264/HEVC VAAPI starts attribute a median 4–5 ms to the help probe and 32–51 ms to complete first-packet delivery including that probe. The probe occupies 9–17% of this narrow startup portion, but saves only a few milliseconds once per start and cannot improve steady-state frames. Retain fresh probing rather than adding cache identity/invalidation state for that measured saving. End-to-end viewer startup was not measured. [Method, results and reproduction](2026-11-21-ffmpeg.md#t1674--native-startup-attribution).

The benchmark uses current production recipes/parsers, waits for the exact child on every observed outcome, and changes no installed session. All six benchmark functions have complexity ≤9; benchmark coverage is exempt. The single opt-in benchmark passes all 60 observations.

## T1620 — display startup/idle integration acceptance

The three scoped outcomes are integrated: absent/failed GPU helpers dispatch zero inspections (T1675), an unchanged 60-frame/one-second idle trace publishes four times instead of 61 while rejection revokes immediately (T1673), and fresh encoder capability probing is retained after native startup attribution (T1674). These are dispatch/publication counts and isolated startup timings; no live CPU or complete viewer-start improvement is claimed.

With all changes present, all 251 tests under `capture::` pass. Three opt-in benchmarks are excluded from the ordinary run: two pre-existing timing experiments and the new T1674 benchmark, which separately passed all 60 native observations. Existing blocked-publication expiry, prompt revocation, viewer epoch and exact child-retirement regressions pass unchanged. The seven maintained production methods across T1673/T1675 retain their measured ≥80% per-function coverage and ≤9 complexity evidence. No new production behavior is added by this integration checkpoint. Active installed sessions remain unchanged. [Integration evidence](artifacts/2026-11-21-open-items/t1620.json).

## T1678 — reduce camera orientation CPU work

Mirror/half turns use row operations; quarter turns reuse a bounded column-offset table. Final 720p/1080p release observations show substantially lower per-frame transform time with matching output checksums. The independent luma/chroma oracles and live-change/invalid-input checks remain permanent. All 36 common camera and 136 host camera/controller tests pass; nine transform production functions have 100% line coverage and all 23 examined functions have complexity ≤9. [Method and timings](2026-11-21-ffmpeg.md#t1678--camera-rotationmirror-kernel).

Each transform owns one additional table of at most 1920 usize entries, with no per-frame allocations or escaping views. Native camera and frame-pool ownership are unchanged. This is source-only; T1577 retains physical feedback and transport acceptance.

## T1490 — bound tuning-cache retention

The Linux cache defaults to 64 records with a configurable 1–256 limit, read on the next serialized write without restarting media. A bounded heap keeps the current name plus the newest other regular cache records. Unrelated names, symlinks and directories are preserved. Busy locks skip persistence; pruning failure prevents publication. Directory descriptors pin eviction and publication to the original directory through pathname replacement, and temporary-file errors are settled before descriptor release.

The original regression observed 65 files instead of 64. A second permanent regression caught redirected deletion in the initial path-based draft; the descriptor-bound adapter passes it and leaves replacement-directory contents unchanged. Config boundary/type/merge tests, cross-process lock exclusion, current-name protection, native failure cleanup and live limit changes are permanent. All 498 common library tests and 43 selection-cache tests pass. Fourteen maintained production methods measure 88.24–100% executable-line coverage; all 101 examined functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1490.json).

The original blocking-worker completion proof remains the cleanup sponsor. Native descriptors are scoped, the inventory heap is bounded, and no tempfile-owning error escapes its directory lifetime. Evicted open inodes retire after their readers close. This bounds normal cache record names/data, not unrelated files or historical unknown temporary names. Source-only: no installed session or configuration changed.

## T1679 — reject obsolete root settings

The root FileConfig schema rejects unknown TOML/JSON keys, including the removed `profile_cache` flag. The original regression accepted that obsolete key before the change. Existing T1194 regressions are retained against the current contract: valid defaults/current tuning controls still round-trip, obsolete values fail parsing, and a locked edit leaves an invalid saved file unchanged. Existing read-error handling still warns and supplies defaults in memory; it does not authorize rewriting invalid saved data.

All 500 common-library tests, 260 GUI unit tests, two GUI process-exit tests and the GUI startup test pass. All 42 measured source/test functions have complexity ≤9. No handwritten production method changed: rejection is generated from the schema attribute and exercised through permanent boundary tests; no standalone generated-method coverage percentage is claimed. Parsing retains no new resource or escaped view. [Evidence](artifacts/2026-11-21-open-items/t1679.json). This is source-only and root-only; T1680 retains the nested settings work.
