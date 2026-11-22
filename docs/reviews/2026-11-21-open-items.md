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

## T1680 — reject unknown nested preferences

CameraProfile, Sensor and host activity settings now reject unknown keys. CameraSettings forwards its current flat layout transparently to the profile contract, so current camera fields and omitted-field defaults retain their meaning. The permanent regression accepted an unknown nested camera key before the change; it now rejects bounded unknown TOML/JSON inputs through direct and FileConfig callers. Known sensor/activity values and complete current round trips still pass.

All 502 common tests and 36 camera-matching GUI tests pass; all 28 examined functions have complexity ≤9. No handwritten production method changed, and no standalone coverage percentage is claimed for Serde-generated methods. This adds no resource owner or escaped parsing value. [Evidence](artifacts/2026-11-21-open-items/t1680.json). Source-only; installed settings and sessions are untouched.

## T1681 — reclaim abandoned current staging files

The cache producer uses an exact owned staging namespace. A new writer first acquires the original directory lock, then removes regular abandoned staging names before creating another temporary record. A live writer keeps the lock through IO and cleanup. No age or PID heuristic authorizes deletion; symlinks, directories and unidentified historical names remain untouched. Errors refuse admission, and descriptor-bound operations preserve a replacement directory.

The permanent regression exits an owned private native child immediately after production staging fsync, bypasses Rust Drop, and waits for that original process before re-admission. Before the fix, the staging file remained; afterward, it is reclaimed. All 47 cache tests pass, including four new staging cases. Twelve production methods measure 85.71–100% executable-line coverage; all 29 measured production/test functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1681.json).

Reclamation retains only one entry/metadata/name at a time and requires a subsequent writer plus successful filesystem operations. Historical unknown names do not acquire guessed ownership. The original blocking worker remains the native completion sponsor; the abrupt-exit hook is test-only. No installed session changed.

## T1682 — capture retry cadence

Crash recovery now verifies original retirement before waiting at the current delay and doubling it for the next attempt. The sequence is 2, 4, 8, 16 and at most 30 seconds; a stable 30-second pipeline resets to 2 seconds. Clean transitions do not wait, and display loss/shutdown interrupts the timer. No new resource owner is introduced. The scalar retry state belongs to CaptureRun; the timer borrows it only during recovery. Unknown native obligations remain owned and prevent replacement.

Three of five new permanent virtual-time tests failed before the behavior change; all five now pass. The full capture selection passes 256 tests with three opt-in benchmarks ignored. The four maintained production functions measure 87.5–100% line coverage, and all 13 measured source/test functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1682.json). These tests use private state and do not start a capture session. Installed binaries remain unchanged.

## T1684 — retain reader outcomes through cleanup

The reader owner stores its consumed outcome before waiting for the original idle controller. A session restart handles a tagged read/assembly error only after cleanup; strict cleanup still returns the error. Task panics and retirement deadlines remain fatal. Cancelling the wait preserves the original outcome for retry. Three permanent tests failed before the change: work failure depended on ready-event order, a joined panic was erased, and work failure returned before held idle cleanup. Two additional boundary tests cover deadline retention and the strict error source chain. Existing T814/T1029/T1046/T1047 assertions remain.

All 261 executed default capture tests and all 126 native-encoder capture tests pass; the default selection excludes three opt-in benchmarks. Eleven maintained production functions, counting the two cfg-specific constructors separately, measure 88.9–100% source-line coverage across both builds. All 73 measured functions have complexity ≤9. Per-build counters remain visible: the shared session body alone measures 60.7% under the optional build and 88.9% under default. [Evidence](artifacts/2026-11-21-open-items/t1684.json).

The optional build used a private extracted Ubuntu FFmpeg 6.1.1 SDK. Its initial run used a stale standalone worker and is explicitly excluded from acceptance; a matching instrumented worker was then built and all 126 tests rerun. No installed library or sharing session changed. The new state retains one bounded outcome slot, without a frame, callback cycle or native lease. Original task/generation cleanup remains authoritative; neither a join nor a deadline settles unconfirmed native ownership. T1685 retains the separately identified GPU fallback race.

## T1685 — GPU fallback follows the settled reader outcome

Natural EOF or reader failure now requests the GPU adapter's one-way FIFO fallback after original cleanup, regardless of the session event that won. Deliberate cancellation leaves the adapter available. Pending idle retirement changes no GPU state, and repeated cleanup cannot replay a consumed completion. The constructor moves the existing task/idle handles into one owner; no new native allocation, frame retention or callback cycle is introduced.

The permanent both-order regression failed before the change. Four GPU decision tests now cover EOF, work failure, cancellation, held retirement and ordinary FIFO completion. All 265 default capture tests and all 126 optional native-encoder capture tests pass; default still has three opt-in benchmarks excluded. Nine maintained production functions measure 88.5–100% source-line coverage across both builds; both constructors are measured separately. The common session body alone is 59.8% in the optional build and 88.5% in default. All 53 measured functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1685.json).

An initial optional coverage collection contained one corrupt native-worker profile despite all tests passing. The invalid profile is retained, no counters were silently omitted, and a fresh full 126-test collection merged successfully. GPU policy fixtures use private tasks and seeded adapter state; this is not physical GPU capture acceptance. Installed binaries and sessions are unchanged.

## T1683 — one authoritative capture teardown

The enclosing authority guard already preempts display-loss/shutdown branches in the reader loop. The supervisor retires the original helper, whose native cleanup disconnects and closes EVDI. Removed the obsolete inner branches, disable-only KScreen helper/matcher, duplicate display-dropped flag and optional session completion. Retained-monitor host-activity quiescence stays distinct from display detach. No new native command or resource owner is introduced.

This resolves unreachable code and stale disable-first prose, not a demonstrated missing native disconnect: the new guarded test and existing C cleanup test passed before the cleanup. T224 retains exact enable/placement and non-KDE command-absence checks; T497 now uses the production authority guard; T372 directly preserves connector identity/enabled-state assertions. T1015 still checks original helper retirement and a fresh generation after detach. T315's three independent C cases require one disconnect and a closed capture handle. No native compositor window-relocation claim is made.

Afterward, all 266 default capture tests, 126 optional native-encoder capture tests and nine KScreen library tests pass, as does the Rust integration test running all three T315 C cases. Default capture has three opt-in benchmarks excluded. Four maintained production functions measure 93.3–100% source-line coverage across both builds; the shared session body alone is 79.7% optional and 100% default. All 92 measured functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1683.json). The private child fixture waits original completion tickets, and existing owners sponsor cleanup on unwind. Installed sessions are unchanged.

## T1617 — integrated capture recovery

T1682, T1683, T1684 and T1685 are complete together: current-delay retry, one authoritative guarded teardown, retained reader outcomes and event-order-independent GPU fallback. The final T1683 suites exercise their combined source: 266 default capture tests, 126 native-encoder capture tests, nine KScreen library tests and one C integration test containing three cleanup cases all pass. Three opt-in default benchmarks remain excluded. Original native retirement remains mandatory. [Integration evidence](artifacts/2026-11-21-open-items/t1617.json). Installed sessions remain unchanged; physical compositor relocation was not measured.

## T1686 — bound original capture acknowledgements

Each admitted capture-control request now retains one absolute deadline across caller cancellation. The host defaults to 2000 ms, configurable with `BLENT_CAPTURE_ACK_MS` from 1 to 60000 ms. Invalid settings reject channel admission. Late acknowledgements cannot certify a transition; expiry fails the channel and uses original-helper recovery. It cannot release native ownership, issue a replacement command or acknowledge quiescence. The child retains the original endpoint lease until actual cleanup.

Three common deadline regressions and one private-helper budget regression failed before the fix. All 506 common tests, 269 default capture tests and 129 optional native-encoder capture tests now pass. Three opt-in default benchmarks remain excluded. The C helper integration test passes all 16 pause/resume transitions with sanitizer checks and clean shutdown. Six maintained production functions measure 94.1–100% line coverage; all 68 measured production/test functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1686.json).

The supervisor regression verifies that an unanswered pause revokes the old generation, retires the original helper, stays paused without replacement, and produces fresh headers/IDR after demand returns. The bounded request/deadline state adds no task, frame buffer or cleanup owner. Tests use private processes and fake EVDI; no physical lock-event acceptance is claimed. Installed binaries and sessions are unchanged.

## T1687 — observe helper failure while parked

The parked supervisor now watches the original helper process and stdout reader alongside demand notifications. Exit or stdout failure retires the original native owners, clears codec headers and the encoder's remembered mode, and returns to waiting. Capture remains stopped until demand is current. The existing original-child/stdout/encoder completion gates still control replacement; process exit or a task result alone does not settle cleanup. No new resource owner or retained frame is introduced.

Both permanent detection regressions failed before the change. Four focused tests now cover child exit, stdout failure, original completion, no-demand waiting, and fresh helper/headers/IDR on resume. All 273 default capture tests and 133 optional native-encoder capture tests pass; three opt-in default benchmarks remain excluded. Both changed production functions measure 100% line coverage independently in each build; all 58 measured functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1687.json). Tests use private native processes; installed binaries and sessions remain unchanged.

## T1618 — integrated parked capture recovery

T1686 and T1687 are complete together: original acknowledgement deadlines survive cancellation, and parked helper failures are observed without waiting for new demand. The final combined source passes 273 default capture tests and 133 optional native-encoder capture tests, including pause receipts, original-cleanup fences and fresh resume. Three opt-in default benchmarks remain excluded. The unchanged common control code also has 506 passing tests and the native C control integration passes 16 transitions. [Integration evidence](artifacts/2026-11-21-open-items/t1618.json). Physical event ordering remains outside these private-process checks; installed sessions are unchanged.

## T1492 — report native encoder packet failures

The optional native encoder worker retains its first packet validation/write failure, rejects further output, and returns the original cause after the synchronous encoder releases its output. The real child now reports a non-zero exit and stderr cause. The output's file and diagnostic share one mutex and bounded owner; completion requires every output clone to return. Rejected packets are returned without retention. No new thread, frame buffer or cleanup sponsor is added.

Two permanent regressions failed before the fix: output accepted another packet after an oversized packet, and the real libx264 child exited successfully after writing to `/dev/full`. Five focused tests now cover those failures, preserved first IO error/source chain, successful framing/encoder result, retained clones and poison. All 16 encoder-worker tests pass. Four changed production functions measure 100% line coverage; native `run` counters come from the instrumented production child, with the other functions measured in the test binary. All 32 measured functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1492.json).

An initial native fixture used generation zero incorrectly and never reached packet output; its log is retained separately, and corrected failing-before evidence is used. The private C producer uses sanitizers. Installed binaries, devices and sessions were untouched.

## T1493 — cache startup baseline (implementation remains open)

An opt-in release probe opens the production cache for a synthetic attachment and private configuration, then loads its saved candidate. Ten open/load observations and ten component observations were collected per FFmpeg runtime. Repeat-call median open time is 70.631 ms with system FFmpeg and 45.316 ms with a private copy of the packaged runtime plus its current manifest; record loading is 0.040/0.030 ms. System FFmpeg's version subprocess accounts for a 52.863 ms component median. Packaged runtime hashing plus the executable hash accounts for approximately 36.8 ms; its version subprocess takes 5.457 ms. Component samples are separate observations, not an exact additive decomposition of each full call.

The executable measured is a 32,566,360-byte release test binary, versus the installed host's 22,568,217 bytes. No filesystem caches were flushed. These are preparation-cost observations, not installed attachment latency or implemented gains. [Baseline evidence](artifacts/2026-11-21-open-items/t1493-baseline.json). Three benchmark functions meet complexity ≤9; benchmark coverage is exempt. The bounded probe retains numeric observations, owns temporary storage and waits original version-query children through the existing scope. Installed sessions are untouched.

T1493 remains open for bounded hash memoization, invalidation regressions and a paired after measurement. Fresh version queries remain necessary for native runtimes whose shared libraries are not completely declared.

## T1493 — bounded runtime digest memoization

The host memoizes file digests using canonical path, device/inode, size, mode and nanosecond modification/change times. Each lookup still opens and checks the current regular file; misses verify the original descriptor after reading. The process-local LRU defaults to 64 entries, supports 0–256 through `BLENT_RUNTIME_HASH_CACHE_ENTRIES`, and retains only paths/stat data/digests. Zero disables retention. Manifest validation, FFmpeg version execution and platform/config observations remain fresh. This depends on coherent filesystem metadata and supplies no authentication or native-ownership proof.

Three alternating repeats per runtime/variant produced 120 private open/load observations. Each table median uses 27 repeat calls; the 12 first calls remain in the evidence and still hash files. No filesystem caches were flushed.

| Runtime | Baseline repeated open | Memoized repeated open | Wall-time reduction |
|---|---:|---:|---:|
| System FFmpeg | 67.497 ms | 52.076 ms | 22.85% |
| Private packaged FFmpeg copy | 44.051 ms | 5.565 ms | 87.37% |

These measure cache preparation in release test binaries, not installed attachment or video latency. [Paired observations](artifacts/2026-11-21-open-items/t1493-paired.json). The system runtime still spends most time in its fresh version query. No first-call speedup is claimed; its baseline outlier is retained.

Two real-filesystem invalidation tests passed before optimization and remain permanent. Deliberately omitting ctime made the same-size/restored-mtime test fail; the correct implementation is restored. The final cache suite passes all 54 executed tests with one opt-in timing probe excluded from the default run. Every admitted capacity 0–256, all metadata fields, replacement/retargeting and invalid configuration are covered; the native descriptor count is unchanged after hashing and unlinking more files than the retention limit. Seven changed production functions measure 94.1–100% line coverage. All 25 measured Rust functions and five benchmark functions have complexity ≤9. [Verification and ownership evidence](artifacts/2026-11-21-open-items/t1493.json).

The original File and bounded read buffer settle within each call; no memo entry retains native work or content. Cache values are discarded on eviction/disable/process exit. The mutex does not span file hashing or native version execution. Installed sessions and binaries remain unchanged.

## T1624 — verify Arch release sources (2026-11-22)

The Arch recipe pins EVDI to the portable build's exact commit and verified archive SHA-256. The release builder downloads the published Blent tag over HTTPS and inserts its measured source checksum into the distributed recipe; source-tree PKGBUILD remains an explicit, unrendered template. Arch CI consumes the generated recipe and uses the commit-named EVDI source directory. Current producers, fixtures and release documentation agree. The AppImage runtime-source portion was already completed under T1622.

All three new regressions failed before the fix. All 29 package/release tests and both real-Cargo recipe tests pass. Final fixture checks also pass both checksum/partial-download tests and the distribution-notices test. Original archive bytes pass the generated checksum; altered bytes fail. Both changed production functions have 100% executable-line coverage (6/6 build lines, 21/21 package lines). All 30 measured functions have complexity ≤9. The actual pinned EVDI archive digest and extraction-root names were checked. [Evidence](artifacts/2026-11-21-open-items/t1624.json).

The original shell owns the temporary directory, waits for the foreground downloader, hashes only after completion, and removes scratch through its EXIT trap. Curl and its file-size resource limit bound the source file to 128 MiB, including unknown-length responses. Partial-download failure leaves no generated recipe or scratch directory. Uncatchable process termination remains outside EXIT-trap cleanup guarantees. Native Arch installation was not executed; installed Blent binaries and sessions remain unchanged.

## T1500 — recover initial activity-source failures (2026-11-22)

The first activity-source creation error now uses the same original-completion gate and backoff as replacement failures. Errors, panics and startup deadlines withhold display demand until fresh evidence arrives. Explicit stop cancels retry admission while retaining any unsettled original native work. No new resource owner is introduced; existing lifetime retention prunes completed attempt certificates.

The new recovery regression failed before the fix. All 13 activity-following tests and the bounded 256-attempt certificate regression pass. The changed production method has 100% line coverage (18/18); all 48 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1500.json). These are injected failures and original-completion tests; installed sessions remain untouched.

## T1513 — align nested shutdown budgets (2026-11-22)

Daemon pipeline observation now covers configured session grace, configured reap observation and a one-second scheduling margin. Defaults yield 6.1 seconds; maximum accepted settings yield 121 seconds. CLI and GUI response ceilings are 126 and 131 seconds respectively, covering the running daemon even when its environment differs from the caller's. Completed shutdown returns immediately. Deadline expiry still cannot settle unknown native obligations or authorize a replacement.

Both new regressions failed before the fix: the real isolated daemon workflow returned after its fixed five-second outer timeout despite a ten-second configured grace, and the client ceiling did not cover the accepted maximum. All 508 common tests, eight daemon-activity tests and both GUI lifecycle tests pass. The exact revised managed-command ceiling also passes its final focused check. Five changed production functions measure 95.4–100% line coverage; all 56 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1513.json). Only private fixture processes were started; installed sessions remain untouched.

## T1609 — use the selected USB runtime for probe ownership (2026-11-22)

The broker passes its selected private runtime to the original helper factory. The native helper opens the directory, creates its bounded per-port lock name with `openat`, closes the directory descriptor and retains the lock until original process exit. It no longer assumes `/run/user/<uid>`. Current direct callers, the development probe, fixtures and documentation use the explicit runtime contract.

Both new native regressions failed before the fix. The 21 native probe/runtime tests, 17 accessory adapter tests, four broker-runtime tests and 207-test Linux adapter coverage run pass. Final expanded runtime checks cover three cases including long paths, full port-name bounds, invalid directories, exact descriptor conservation, contention and reuse after original exit; the final broker-path forwarding regression passes. The production helper compiles with cached libusb headers and warnings as errors; the development example passes its compile check. All seven changed production functions have 100% line coverage; all 57 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1609.json). No physical USB transfer or installed session was touched.

## T1600 — fund broker clients and reject overload explicitly (2026-11-22)

The broker now defaults to 128 clients, four links and 64 registrations. Each registration consumes a client; media bridges use separate loopback streams, correcting the original ledger's unsupported three-tablet claim. A broker-owned task pool funds the selected client capacity independently of the generic 64-task pool. On-demand launches pass validated client/link/registration capacity settings; the standalone CLI uses the same defaults.

Excess admission starts one server-owned rejection exchange with the existing I/O deadline. It drops received descriptors before sending request-bound Busy, never admits a route, and continues polling existing clients. Further accepts wait in the bounded socket backlog. Shutdown drops the original exchange synchronously. Admitted peers retain their exact task admission through destruction and join; no timeout or socket EOF substitutes for remote retirement.

Three new regression cases failed before implementation; strengthening the admission case to request 68 clients exposed the separate 64-task ceiling. All 287 broker tests now pass, including seven T1600 tests. Real registration clients reuse a single slot across three Busy responses, with both listener addresses available again after each original retirement. Silent excess clients cannot block admitted controls or shutdown. All 18 changed production functions measure 92.3–100% line coverage; all 158 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1600.json). Private fixtures only; no physical USB transfer or installed session was touched.

## T1688 — suppress redundant EVDI fallback grabs (2026-11-22)

Every completed framebuffer grab now resets the existing fallback deadline, including an empty grab. Ordinary update events therefore prevent an extra grab each second; watchdog recovery and the no-event fallback remain available. The capture event loop remains the sole deadline owner; allocation, native registration, writer borrowing and cleanup boundaries are unchanged.

The permanent regression failed before the fix. All 72 helper tests pass, including the new ASan/UBSan case covering 240 ordinary update callbacks and the subsequent 999/1000 ms fallback boundary. The changed function has 100% executable-line coverage (24/24); all measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1688.json). This is deterministic mocked-EVDI evidence; no physical performance claim or installed-session change.

## T1690 — disclose the latency sample window (2026-11-22)

Capture-to-FIFO logs now identify the first eligible samples and the 256-sample cap per report. Current benchmark parsing preserves both sample count and capacity and rejects the obsolete frame-count form. Documentation describes the 0–999.999 ms eligibility range and potentially delayed reporting interval; these are not all-frame percentiles.

Both the native log and parser regressions failed before the fix. The native log regression and all 12 benchmark/parser/plot tests pass. The changed production function measures 90% line coverage (18/20); all 96 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1690.json). No ownership or allocation boundary changed. The separate two-sample quantile inconsistency remains tracked as T1692.

## T1692 — use consistent empirical percentile ranks (2026-11-22)

Both capture percentiles now select the first sorted sample whose cumulative fraction reaches the requested percentage. The old mixed indexing logged p50=8 ms and p95=7 ms for samples [7,8]; the new result is p50=7 ms and p95=8 ms. Invalid rank inputs reject before indexing.

The actual log regression failed before the fix. Both native tests pass under ASan/UBSan, including 26,418 bounded count/percentile pairs against an independently iterated rank oracle and two extreme invalid inputs. All 12 benchmark/parser tests pass. Both changed functions measure 100% line coverage; all 90 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1692.json). Pure bounded arithmetic uses the reporter's existing stack snapshot; no ownership boundary changes.

## T1691 — remove historical benchmark label aliases (2026-11-22)

Current summary and plot readers no longer interpret obsolete encode→display or decode+render labels. They retain the current host-clock and tablet-clock boundaries. Permanent tests now require old-label rejection while retaining current-field, independent-window and no-derived-wire oracles; historical reports remain unchanged.

Both rejection regressions failed before removal. All 12 benchmark/plot tests pass; all 25 measured functions meet complexity ≤9. Constants-only benchmark changes are within the existing coverage exemption and introduce no resource lifetime. [Evidence](artifacts/2026-11-21-open-items/t1691.json).

## T1689 — preserve absolute capture request cadence (2026-11-22)

EVDI requests now advance an absolute nanosecond phase. The native poll wait alone rounds up to milliseconds; late observations skip missed deadlines without issuing catch-up bursts. Actual request time remains separate for watchdog recovery, and resume starts a fresh phase after its original snapshot. Current callers and the obsolete clock_nanosleep comment were updated together.

Both ten-second cadence regressions failed before the fix: configured 60/90 FPS produced 589/834 requests. They now produce 600/900 respectively. These are scripted clock/native-request counts, not measured hardware throughput. All 78 helper tests and 14 conversion/module/retirement tests pass; final-source counters additionally execute 17 existing native cases. Boundary checks cover 24,000 late-phase steps across 1–240 FPS, eight poll rounding/saturation inputs and 100 same-time retries producing one request. Existing invalid-FPS, long-uptime, pause, lost-event and native-storage tests remain. All 10 changed production functions measure 96–100% line coverage; all 186 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1689.json).

The deadline scalar is owned only by the original capture event loop. No allocation or native lifetime was added, and no installed session was touched.

## T1608 — capture cadence/statistics integration (2026-11-22)

The three original outcomes are complete: T1688 (`7d850a5e`) suppresses redundant fallback grabs, T1690 (`d3b90684`) discloses the bounded latency sample window, and T1689 (`d3904ea7`) preserves absolute request cadence. The additional percentile inconsistency is fixed separately by T1692 (`33fc7dac`); historical benchmark label aliases were removed under T1691 (`0f8528d0`).

The final 78-test helper suite covers their combined behavior. All 14 conversion/module/retirement tests also pass; current helper callers and benchmark readers agree with the new contracts. Per-function coverage and complexity evidence remain in the child artifacts. Native throughput and live kernel-log improvements were not measured; installed binaries and sessions remain untouched.

## T1693 — expose paired host stream settings (2026-11-22)

Host greetings now include bitrate and FPS from the same encoder-settings read guard. Without a settings source, neither field is invented. Android validates the paired values and retains one immutable observation tied to its original control notice; cutoff clears it and stale socket callbacks cannot replace it. The shared greeting fixture and protocol documentation use the current contract.

The missing-bitrate regression failed before the fix. All 49 host input-contract tests and 84 Android API-27/34 cases pass, including eight new observation cases and 1,000 concurrent host snapshot checks. All eight maintained functions/constructors measure 100% line coverage; all 123 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1693.json).

The observation retains only bounded values and a delivery identity; it adds no native resource or worker. T1694 still must connect it to draft settings and explicit override intent. T1569 remains incomplete until that UI/reconnect behavior is verified.

## T1694 — retain only explicit tablet stream overrides (2026-11-22)

Current paired host observations seed the tablet settings. Apply sends only edited fields; unchanged Apply sends nothing. Explicit fields persist independently and reconnect resends only those fields. Use host settings removes stored and retained intent and invalidates old settings notices; it does not cancel writes already accepted by the socket or undo host changes. Rejection restores confirmed values only for previously explicit fields. Old preference APIs/keys have no migration or fallback.

All eight original UI regression cases failed before the change. All 333 selected Android API-27/34 cases now pass, including 34 new T1694 cases for sparse wire/persistence, host refresh, draft edits, reconnect, clear, stale responses and invalid boundaries. Existing short-landscape Apply and camera ownership tests retain real edited Apply actions; generation and rejection oracles remain. All 71 maintained functions/constructors/callbacks measure 100% line coverage; all 871 measured functions meet complexity ≤9 (maximum 8). [Evidence](artifacts/2026-11-21-open-items/t1694.json).

The new domain values are bounded immutable records. Control intent and the submitted request remain under the original controller lock; persistence and UI have explicit separate lifetimes. No native worker/resource was added. Tests use Robolectric and fake sockets; installed sessions and the tablet were not changed.

## T1569 — host settings and sparse tablet intent integration (2026-11-22)

T1693 (`bd94f3d2`) exposes a paired host observation; T1694 (`417deccc`) connects it to sparse edits, persistence, clear and reconnect. All 49 host input-contract tests pass again against the final tree. The integrated Android suite passes all 333 selected cases, including 34 new sparse-intent cases and the retained stale-response, generation-exhaustion, camera-ownership and display-lifecycle checks. Per-function coverage and complexity are recorded in the child artifacts. No installed session or tablet was changed.

## T1619 — request a fresh keyframe after viewer lag (2026-11-22)

Both asynchronous receive lag and batch-drain lag now raise the existing encoder refresh signal while restoring the viewer's IDR wait state. A shared helper keeps the two paths consistent. The viewer retains the existing bounded shared atomic signal; no new task, native handle or frame storage was added.

Both regressions failed before the behavior change. All 51 stream tests pass; the existing T784 opt-in TCP timing benchmark remains ignored. New cases verify both refresh paths, no repeat request for ordinary packets, newest-IDR selection, closed-producer completion and zero retained batch bytes. All five changed functions measure 100% line coverage; all 72 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1619.json). The optional in-process encoder consumes this request; CLI FFmpeg keeps its periodic schedule. No physical performance gain is claimed.

## T1572 — follow display rotation during camera capture (2026-11-22)

An application-owned Android display observer now notifies the camera policy when the default display changes. The current camera is replaced through its existing retirement lane with an immutable angle snapshot. Duplicate/invalid angles and inactive cameras do not start workers; rapid changes retain only the latest pending angle. Explicit background sharing continues to follow display changes. Rotation briefly restarts capture; the wire remains greeting-based.

Two portable regressions failed before the change. All 267 selected camera/resource-lifecycle cases pass, including five new portable cases and four API-27/34 adapter/composition cases. The fake Binder test requires original worker death before replacement and checks the replacement task's orientation. All 22 maintained functions/constructors/callbacks measure 100% line coverage; all 114 measured functions meet complexity ≤9 (maximum 7). [Evidence](artifacts/2026-11-21-open-items/t1572.json).

The process owns one listener; cutoff fences queued callbacks before unregistering its exact identity. Existing resource, generation, permission and media-consumer ownership remain authoritative. The listener retains only application context, and native capture still belongs to the isolated worker. Physical tablet rotation and interruption duration were not measured; installed sessions remain unchanged.

## T1603 — deliver pending permission status to registering clients (2026-11-22)

The broker now forwards PermissionRequired with the original registration request and route. The current wire enum and client phase retain that pending state without admitting media or releasing ownership. Closing suppresses late server notices; the client rejects regression from active/terminal states and does not undo local cancellation. The bridge's no-op is intentional: pending permission must not open local I/O.

Both client-contract regressions and the broker delivery regression failed first. All 288 broker, 17 registered-display, 13 client-model and five wire-contract tests pass. The independent transition matrix covers all 36 state pairs; the wire test checks all 256 byte values. All five changed production functions measure 87.5–100% line coverage; all 98 measured functions meet complexity ≤9 (maximum 8). [Evidence](artifacts/2026-11-21-open-items/t1603.json). No new worker, native handle or cleanup obligation was added. Tests used private helper/TLS/IPC fixtures, without contacting installed Blent or the tablet.

## T1697 — serialize private ADB fixture routes (2026-11-22)

A held fake removal previously read its route snapshot before waiting, so another removal could be overwritten and a retired route resurrected. The fixture now waits first and serializes current reads/mutations with one stable file lock. Each command owns and closes its descriptor; no production or installed ADB state changes.

The permanent delayed-removal test fails before and passes after the fix. All five measured fixture/test functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1697.json). The namespace drain test still exposes a separate deadline mismatch, tracked as T1698; this fixture correction alone does not close that integration check.

## T1698 — preserve the intended native-drain fixture ordering (2026-11-22)

T1513's default 6100 ms body observation budget exceeds the ADB command's 5000 ms deadline. The private T1509 fixture consequently killed its held original command before releasing it, leaving an intentionally unknown effect. It now explicitly selects 100 ms session grace and 100 ms reap, preserving the one-second margin. It also verifies that the original fake removal is still alive when entry drain starts.

The existing permanent T1509 test fails before and passes after this fixture correction; all three existing namespace tests pass. Original PID, repeated-signal, exact route settlement and CLI command-absence assertions remain. All four measured fixture functions have complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1698.json). No production deadline, cleanup proof or installed process changed.

## T1699 — retarget the Stop deadline oracle (2026-11-22)

The permanent T497 Stop test still expected the obsolete ten-second diagnostic after T1513 enlarged the client observation budget. It now independently checks the accepted 126-second bound and elapsed virtual time, retaining its assertions that the original nonresponsive daemon remains alive and is explicitly killed/waited only by the fixture. The test fails before and passes after; every measured function in the fixture file has complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1699.json). No production behavior changed.

## T1695 — arbitrate daemon startup with one retained lease (2026-11-22)

Startup now acquires an exclusive regular-file lock in the validated runtime directory before publishing daemon state. The entry retains the original descriptor with its PID publication until native exit drain completes; the lock inode is never unlinked. The process-scan startup veto was removed. Existing same-user discovery remains for status and Stop.

The real two-contender regression fails before the fix: a daemon blocked reading configuration prevents the ready contender from starting. Afterward, all 33 selected binary tests and all four namespace integration tests pass. They cover concurrent admission, rejection while active and draining, settled reuse, invalid lock objects, failed/partial publication, cancellation and unwind. Fixture defects exposed by broader verification were resolved separately as T1697–T1699, preserving their original ownership assertions.

All four measured production functions have 94.4–100% line coverage, including 96.3% for the actual daemon startup workflow; all 129 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1695.json). Tests use private PID/device namespaces and fake ADB. Installed sessions remain unchanged. T1512 still requires original-process Stop handling under T1696.

## T1696 — stop the original daemon through a pinned handle (2026-11-22)

Stop now captures validated daemon snapshots and reuses the Linux pidfd adapter. Every candidate is pinned and revalidated before any SIGTERM is sent; exit observation uses those same handles. A replaced identity is ignored. The client keeps its existing bounded response budget and never escalates to SIGKILL. The now-unused local PID predicate wrapper was removed, with its test retargeted to the shared predicate.

The stale-generation native-child regression fails before and passes after the fix. All 76 common Linux tests, eight final host binary tests and four namespace integration tests pass. The final integration uses the real Stop CLI against its private daemon. Tests retain delayed cleanup, nonresponsive-daemon survival, foreign identity rejection, invalid/dead handles, missing/stale PID-file recovery and separate capture-orphan escalation. An earlier library filter matched zero orphan tests; the final binary run executes all three and supplies their counters.

All nine maintained production functions measure 93.3–100% line coverage; all 157 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1696.json). Observation descriptors close on partial failure, cancellation and timeout without claiming target cleanup. The test models PID reuse using a stale start tick; it does not force native PID wraparound. Installed sessions remain untouched.

## T1512 — daemon startup and Stop integration (2026-11-22)

T1695 (`294731ae`) retains one startup lease through native exit drain; T1696 (`a06c1899`) signals and observes original process handles. The final eight binary tests and four namespace integration tests exercise both changes together, including real CLI Stop after settled startup-lease reuse. Shared Linux adapter verification passes all 76 selected tests. Child artifacts retain per-function metrics, failure controls and native scope. No installed daemon or device was changed.

## T1554 — Autostart changes future logins only (2026-11-22)

Per the owner's selected behavior, the Linux checkbox now only updates login registration. Its daemon-discovery/start/stop side effects and the obsolete injected running-state wrapper were removed. Explicit lifecycle controls continue to manage the current daemon. Installation and architecture instructions describe the same behavior.

The permanent T231 regression fails before and passes after the change. All three GUI autostart tests and both shared autostart tests pass. Private fixtures cover desktop/service routes, absent or failed managers, preserved entries on unreachable-enabled-service errors, no daemon actions on either toggle, and survival of an unrelated live daemon. The changed production function has 100% line coverage; all 201 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1554.json). No installed preferences or sessions changed; Windows work remains paused.

## T1700 — expose the latency snapshot bound to optimized builds (2026-11-22)

The strict production build exposed GCC's inability to infer the sample-capacity contract across translation units: the percentile helper can return -1 for an oversized count, and reporting checked only that the count was positive. Reporting now also checks the existing capacity bound before sorting/indexing. The producer already guarantees that bound; this is a confirmed build failure, not a demonstrated reachable out-of-bounds read.

The permanent GCC 13.3 `-O3 -std=c11 -Wall -Wextra -Werror` compile test fails before and passes after the fix. All three existing native latency/log/rank tests pass with sanitizer checks, and the reporting function has 100% executable-line coverage (20/20). All 62 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1700.json). The bounded stack snapshot and capture-loop lifetime are unchanged; no device or installed process was accessed.

## T1611 — share the strict EVDI production build (2026-11-22)

Local Make, portable-release and Arch builds now invoke `host/evdi/Makefile`, which owns the eleven-source list and C11/Wall/Wextra/Werror policy with generic O3 by default. Explicit output and library paths survive spaces, quotes and literal shell characters. Existing Make dry-run/jobserver behavior, pinned EVDI source checks and replaceable-library packaging remain intact.

The strict-flag regression fails before and passes after the change. All two shared-recipe, two Arch, eight Make, two distribution and nine package tests pass; the standalone production-optimization compile regression also passes. Existing module-link, source-integrity, literal-path, APK-rejection and missing-input assertions were retargeted to the shared recipe without weakening them. The native helper compiles and links with GCC 13.3.0 and retains `libevdi.so.1` plus `$ORIGIN` RUNPATH. The newly exposed optimized warning was fixed separately as T1700 (`0a38ad32`).

The maintained Arch build function has 100% native Bash line coverage (6/6); all 40 measured shell/Python functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1611.json). Portable/Arch caller checks use private fixtures; no full native package installation or physical capture acceptance is claimed. Installed binaries and sessions are unchanged.

## T1610 — reject invalid helper options consistently (2026-11-22)

The helper rejects FPS outside 10–90, scale outside 1–4, conversion workers outside 0–128 and inherited descriptors below 3. Card indices retain -1 for automatic selection. Malformed options return status 2 before native admission; numerically valid descriptors still need a suitable live socket. Defaults remain 60 FPS, scale 1 and automatic workers. Usage lists every accepted option and its bounds, with a permanent check against the shared Rust product limits.

All three new regressions fail before and pass after the fix. All 81 native helper tests pass, including 1,325 numeric option cases, 16 malformed/reserved descriptor cases, two unusable-descriptor cases, and the retained bounded text/number fuzz and worker-failure tests. The native helper also builds with GCC 13.3.0 at C11/O3/Wall/Wextra/Werror. All 11 maintained production functions meet the 80% line-coverage requirement; all measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1610.json).

Parsing retains only process-lifetime argument borrows and scalar settings. Invalid options admit no native work; existing socket and worker ownership is unchanged. No installed sessions or physical device acceptance were involved.

## T1701 — resolve the EDID cache through the platform adapter (2026-11-22)

Generated EDIDs now use the shared data-directory resolver. Missing, empty and relative HOME fail before creating directories; an absolute non-UTF-8 HOME is preserved. The isolated subprocess regression fails before and passes after the fix. All 19 EDID tests pass, the changed production function has 100% line coverage (11/11), and all 30 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1701.json).

The existing 128-byte generator and atomic file replacement retain their owners and cleanup. Every fixture child settles before its private directory is removed; no installed data or device is touched. T1621 retains cache-name, helper-rate and unused-API integration work.

## T1702 — identify EDID cache entries by mode and instance (2026-11-22)

The generator-version suffix and its obsolete rationale are removed. Current cache names depend on display instance, geometry, refresh and physical size; full content comparison decides reuse or atomic repair. Existing files are neither migrated nor deleted.

Retargeted permanent T114/T115 regressions fail before and pass after the change: valid-checksum files with wrong physical size or refresh range are repaired at the same current path, with one cache entry retained. All 19 EDID tests pass, including incomplete-content repair, matching-inode reuse and failed replacement. The maintained function has 80% executable-line coverage (24/30; unexecuted tracing arguments remain reported); all 27 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1702.json). Bounded generator buffers and atomic temporary-file ownership are unchanged; installed files and sessions were untouched.

## Structural review queue (2026-11-22)

Static screening examined 3,877 production functions in 504 Rust files and found no exact whole-body duplicates above its 180-normalized-character threshold. That is candidate screening, not proof of architectural consistency. Manual Rust, Android, native-C and packaging review confirmed five structural outcomes: controller reply construction (T1705), shared Binder worker lifetime ownership (T1706), production EDID encoding (T1707), capture mode-change classification (T1708), and EVDI cadence separation (T1709, child of T1613). Reviewers executed no project tests, builds or native operations.

T1613 now retains separate cadence, readiness, idle-advice and GPU-query outcomes; T1625 retains independent service-template and shared-system-setup outcomes. The queue sequences current EDID cleanup before encoder consolidation, capture policy before readiness optimization, Android dead-path cleanup before shrinking, tunnel handoffs before allocation work, and broker polling before reaper measurements. These preferences do not invent prerequisite failures. Original lifecycle fences, native acceptance obligations, permanent independent oracles and category pauses remain. No performance gains or runtime bugs are claimed from this structural review.

The daemon composition root was reviewed and not flagged merely for length. T1536's unused-data-directory assertion was corrected after T1701 introduced a current production caller. [Scope, candidates and queue](artifacts/2026-11-21-open-items/structure-2026-11-22.json). All confirmed outcomes are recorded in the domain ledger.

## T1703 — pass the selected refresh directly to EDID validation (2026-11-22)

Helper command construction no longer clamps generated refresh to 90 or carries the obsolete 120-Hz rationale. The generator receives the selected FPS and rejects invalid product modes. The existing private command fixture now uses valid 90-FPS input and permanently checks 0, 9, 91, 120 and u32::MAX rejection; it fails before on 91 and passes after. Explicit EDID paths and card priority retain their behavior.

All 38 helper tests pass. The maintained command function has 94.9% line coverage (37/39); all 36 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1703.json). Command construction spawns no child; existing cache/argument ownership is unchanged. Installed sessions were untouched.

## T1715 — correct metric parser attribution (2026-11-22)

T1610/T1701/T1702 artifact labels incorrectly named tree-sitter for Rust. The checked-in runner uses the locked syn 2.0.119 frontend; only C uses tree-sitter 0.25.2. Labels now name those actual tools. Function counters and results are unchanged; this documentation correction claims no additional execution.

## T1716 — retarget selector preparation ownership fixtures (2026-11-22)

The T1704 baseline exposed two stale regressions: T1037/T1050 waited for a blocking FIFO fingerprint read, but current hashing opens nonblocking and rejects non-regular files. Both failed at FIFO writer admission before T1704 production changes. Their private helper is now a regular file, with a test-only per-path barrier in the actual blocking preparation thread. The registry admits at most two original guards; dropping a guard removes only its matching identity and disconnects the waiter, including unwind.

Both previously failing tests pass, as do all three selected cache-context regressions (five executed cases total). No-overlap, original lifetime retained after parent cancellation, late native child publication and quiescence fences retain their original assertions. The containing host-stamp function measures 96.7% line coverage (29/30), and all 53 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1716.json). Production hashing/acceptance is unchanged; the barrier is compiled only for tests. No installed session or physical device changed.

## T1704 — remove unused production API and duplicate subscription wrappers (2026-11-22)

The default-size EDID wrapper is removed; tests call the current sized API. StreamServer's unused public Stop method, private watch channel and unreachable graceful-stop branch are removed. Production already cancels the owned server future; permanent fixtures now exercise that path through standard cancellation. They retain stop-before-first-poll, immediate wake, descriptor-exhaustion recovery, original socket closure and subscription-release assertions. Viewer admission now has one explicit optional-capability method; no old-signature or named compatibility wrapper remains. Missing capability continues to deny idle-capture qualification.

All 51 retargeted stream baseline tests passed before removal. Final verification passes 71 library and 73 binary tests, with one existing opt-in benchmark excluded. Two stale selector preparation fixtures exposed by the broader baseline were fixed independently under T1716. All four maintained production functions measure 100% line coverage; all 302 measured functions meet complexity ≤9. Coverage was exported from exactly the two executed unit-test binaries to exclude stale build-object maps. [Evidence](artifacts/2026-11-21-open-items/t1704.json). Existing listener/client, viewer epoch, presence and media-budget ownership is preserved; no installed session or physical device changed.

## T1621 — EDID and stream cleanup integration (2026-11-22)

T1701 (`8fb9866e`) validates the cache location; T1702 (`a9efefec`) removes versioned filenames; T1703 (`77cdc3db`) passes refresh unchanged to validation; T1704 (`37062396`) removes unused APIs and duplicate subscription wrappers. Final integration passes all 19 EDID tests alongside the 71 library and 73 binary cases recorded for T1704. Child artifacts retain individual coverage, complexity and ownership evidence. No installed session or device was changed.

## T1717 — share the EDID contract and runtime encoder (2026-11-22)

A platform-independent `blent_config::edid::Mode` request now owns geometry, panel size, refresh and instance identity. Its encoder returns an owned 128-byte array and borrows at most twelve name characters; current custom-name replacement, termination and padding semantics are retained. The host owns only cache placement/publication. No storage or native backend type enters the shared contract.

All seven selected shared encoder/timing tests and all 19 host EDID tests pass. Existing independent physical-size, color, range, checksum, overflow and cache-repair assertions remain; the Python producer still participates until T1718. All four maintained production functions have 84.2–100% line coverage, and all 32 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1717.json). Successful payload storage is fixed-size; returned diagnostic errors may allocate. No borrowed output or native work escapes, and the host's atomic cache ownership is unchanged. T1707 remains open for development-tool migration.

## T1718 — use the shared encoder in the development CLI (2026-11-22)

`blent-edid` replaces the duplicate Python production writer; Make, current documentation, coverage inventory and the bounded mutation catalog use the native producer. The runtime cache and development CLI now call the same scalar-request encoder. The retained Python regression suite still exercises its 1,048-case numeric corpus against the real CLI. File-vs-encoder equality, independent decoded fields, custom names, default arguments, invalid-request preservation and output errors are permanent native integration tests.

All four CLI integration tests, 19 host EDID tests, three Python regression tests, 12 inventory tests and the install-recipe test pass. The mutation tooling suite passes 26 tests with one preexisting foreign-platform skip. Both targeted checksum and physical-width faults compile and fail the intended independent assertions in a private source copy; baseline and restored tests pass. All three new production functions have 100% line coverage; all 104 measured functions meet complexity ≤9. [Evidence](artifacts/2026-11-21-open-items/t1718.json).

Requests validate before output opening. Successful encoding owns one 128-byte block; the synchronous write closes its descriptor on success/error. I/O failure can still partially modify an existing output, as before; no atomic-write claim is introduced. Test children settle before temporary directory cleanup. No installed sessions or devices changed. T1719 separately tracks progress-blind mutation-runner caps; those runners were not used for these deliberate controls.

## T1707 — shared EDID producer integration (2026-11-22)

T1717 (`e11fe0fd`) establishes the independent shared encoder and runtime cache adapter; T1718 (`29309326`) moves the development producer, current Make/docs/inventory and mutation controls to it. The duplicate production writer is removed. Final evidence is the retained 19 host tests, four native CLI tests, three Python corpus tests and both caught deliberate faults in the child artifacts. This consolidation claims no measured performance gain and changes no installed sessions.
