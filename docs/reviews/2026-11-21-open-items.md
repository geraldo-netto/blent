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
