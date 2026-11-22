# UI/UX implementation — 2026-11-22

Scope: owner-authorized T1780–T1792; one verified commit per item, reinstall/reload
once the batch is complete. No push. Native appearance, navigation/context,
settings lifecycle, accessible presentation and Android scope form the dependency
order; independent contrast fixes need not wait for appearance research.

## T1781 — settings parent navigation

A pure `Tab::section` projection separates the visible parent from the active child.
Selecting another section changes the route; rendering/selecting the current parent
preserves the child. General remains first. No persisted or native state changes.

The permanent visual regression failed on the original implementation (missing
parent selection on a child) and passes after the fix. All 14 Overview tests pass.
Exact executed GUI object LLVM counters: section 5/5 lines, workspace navigation
26/26, section controls 12/12. Rust 1.100.0, cargo-llvm-cov 0.8.7; local syn 2.0.119
checker measured 194 functions across main/Overview tests, maximum complexity 9.
[Evidence](artifacts/2026-11-22-ui-ux/t1781/).

Lifetime: frame-local enum projection and borrowed UI; egui owns shapes for the
frame/context. No allocations, queues, callbacks, native leases or new cleanup
obligations escape the existing UI owner. Native batch acceptance follows deployment.

## T1780 — selected and error text contrast

Remove the global foreground override so selected controls use their semantic
selected-text color. Validation uses the theme error role, with a readable light
error default. Both permanent contrast regressions failed before the change and
pass after it, testing normal text against its actual light surface at >=4.5:1.
The validation fixture covers valid and invalid drafts. These are enabled text
checks, not an assertion that every disabled widget must meet the same threshold.

Both changed production functions have full measured line coverage; the touched
source/test files pass complexity <=9. Tools match T1781.
[Evidence](artifacts/2026-11-22-ui-ux/t1780/). Style scalars remain context-owned;
there is no additional native resource, retention or asynchronous work.

## T1789 — exact camera configuration target

Configure records one UI target without editing saved/default profiles. The next
start uses that identity; live adjustments/restart still use the admitted owner.
The page distinguishes active and next targets, identifies the route, and lets the
user explicitly return to saved target selection. Display preferences are labelled
as affecting all tablets. ADB identity alone does not prove USB versus network ADB.

The permanent route regression failed by starting `first` after configuring
`second`. Four target tests now pass, covering ADB/USB, active-versus-next target,
invalid/automatic saved selection, and explicit context clearing. The broader run
passed 266 existing/target tests and exposed one newly authored fixture that used
prepare without admission; corrected to an acknowledged admission, that test
passes in the focused four-test run. No production behavior was changed to satisfy
that fixture correction. A full final GUI run follows the batch.

All 13 changed/new production methods measure 92.3–100% line coverage; 293 functions
in the affected source/tests have complexity <=9. Tools match T1781.
[Evidence](artifacts/2026-11-22-ui-ux/t1789/). Context owns a single validated identity
and display name from the bounded workspace; replacement/drop frees those values.
It holds no native lease, frame, borrowed widget or callback. Existing backend
admission, original-task settlement and stop/reuse boundaries remain authoritative.

## T1788 — camera preview and controls

The stopped preview reserves only 48 points instead of a full video rectangle.
At >=980 available points, the live preview keeps its 640-point area with image
controls beside it; narrower layouts stack. Camera pages may use up to 1280 points.
Sensor adjustments have their own disclosure beside basic image controls, separate
from advanced transport/worker tuning.

The original default-size regression failed because clipped image controls were
not rendered; it now sees Start, Lens, resolution and both shadow controls above
the footer. Additional fixtures cover 380/1040-wide live layouts and discovering
sensor controls while stopped. All 43 camera-filtered tests pass; the stopped and
sensor regressions also passed in the focused run. Five changed methods measure
85.7–100% line coverage; 251 measured functions have complexity <=9.
[Evidence](artifacts/2026-11-22-ui-ux/t1788/). The existing single Arc thumbnail and
texture remain context-owned and expire through the same maintenance path; layout
borrows them per frame, adds no capture and preserves aspect ratio. Native scaled
layout acceptance follows the combined installation.

## T1790 — connection route guidance

Connections explains Direct USB and ADB separately, shows observed ADB tablet names
and links back to Tablets. Missing ADB offers an actionable installation/direct-USB
choice. ADB identity is not presented as proof of a physical USB connection.
The direct-USB panel retains its existing capability and permission behavior.

The permanent missing-route regression failed before the change and passes after
it. 33 connection-filtered, 2 focused route and 14 Overview tests pass. All five
maintained methods measure 90.9–100% line coverage; 212 measured functions have
complexity <=9. [Evidence](artifacts/2026-11-22-ui-ux/t1790/).
Guidance borrows the existing bounded device snapshot for one frame; it retains no
endpoint or helper and issues no connect/disconnect/mode-switch command.

## T1792 — General preference hierarchy

General shows ordinary preferences before the service section; required setup
still comes first when unavailable. The service command is a normal compact
button with its all-tablets scope intact. CPU scheduling remains under Advanced
app settings. General stays first and no command/persistence behavior changes.

The original default-size regression could not find the clipped login control.
Both new layout/disclosure tests pass; the existing private-command setup/service
lifecycle test passes. Three changed methods measure 81.0–100% line coverage;
190 measured functions have complexity <=9.
[Evidence](artifacts/2026-11-22-ui-ux/t1792/). Widget borrows remain frame-local;
existing action workers and service ownership are unchanged.

## T1791 — contextual action recovery

Each action receipt owns its original operation and feedback area. Completion and
lost-result messages retain that identity, distinguish an unconfirmed effect from
failure, and name a supported next check. Resource errors expose recovery guidance
with expandable diagnostics. Admission and native retirement are unchanged; losing
an already-disconnected receipt does not settle delegated effects.

The original recovery regression failed; three targeted regressions pass, including
pending/completed identity and diagnostic disclosure. The full GUI run passed
274/278 tests: two exact-message fixtures needed the intentional operation prefix
and now pass; two responsive-layout failures are tracked separately as T1793.
The private-command service/setup regression also passes. All 14 maintained
production methods measure 86.7–100% line coverage; 289 measured functions have
complexity <=9. [Evidence](artifacts/2026-11-22-ui-ux/t1791/).

One bounded operation string travels with the existing single receiver. It is
released with that receipt on completion/disconnection; no worker, queue or native
lease is added. Resource diagnostics remain borrowed during the frame.

## T1793 — responsive test geometry follow-up

Full-suite verification exposed a test-oracle error: centered text anchors were
interpreted as top-left corners. The permanent left/center/right geometry
regression failed before correcting the helper to translate the actual galley
bounds. Native clipping was not established; no production preview change was
needed. Responsive fixtures now use the visible Updates and Direct USB anchors.
The geometry, 380-point clipping and five-width alignment tests all pass, without
weakening bounds. Helper coverage is 80% (8/10 executable lines); all measured functions have complexity
<=9. [Evidence](artifacts/2026-11-22-ui-ux/t1793/). Borrowed immutable shape geometry
is inspected synchronously; the test collector owns only its bounded label list.

## T1782 — global draft scope

The footer remains visible outside Settings while edits or transactions exist.
It names the count of unsaved fields across all settings, exposes field/page links
in a bounded review popup, and states when saving restarts every tablet display.
Hidden invalid fields remain reachable and disable Save. The global Save/Discard
transaction semantics are unchanged. The technical config-path footer was removed
to preserve space for the actionable summary; camera basics still fit the default
window. No configuration values are exposed in the summary.

Two original regressions failed before implementation. Nine appearance tests,
16 Overview tests, two leaf-diff tests, the shared-Save test and three save-lifecycle
tests pass. Coverage for all 11 maintained methods is 87.5–100%; 205 measured
functions have complexity <=9. [Evidence](artifacts/2026-11-22-ui-ux/t1782/).

The summary is a frame-local projection of the existing draft and saved snapshot;
its leaf count/depth follows the fixed configuration schema. It owns temporary
strings and tables only, freed after rendering. Popup expansion belongs to egui;
no second draft, transaction, native resource or subscription is created. The TOML
dependency already used by Blent is promoted from GUI test-only use, without a new
version or package.

## T1783 — explicit close decisions

Dirty or busy windows cancel native close and offer Save preferences and close,
Discard and close, or Keep editing. Save from this prompt explicitly avoids a
display restart. Pending original saves cannot be discarded, duplicated or lost;
Keep editing dismisses the prompt while their work continues. Failure retains the
draft and prompt. Active media still minimizes; its original owner is retained.

The close regression failed before implementation. All 12 close/media lifecycle
tests pass, including the private X11 close/reactivation fixture. A separate private
native startup test executes the production constructor with isolated config/runtime
paths and a local curl stub, then closes a clean window without a prompt. Existing
media-close fixtures now explicitly start clean, preserving their original exit
oracle; dirty behavior has separate permanent coverage. All ten maintained methods
measure 95.7–100% line coverage; 219 measured functions have complexity <=9.
[Evidence](artifacts/2026-11-22-ui-ux/t1783/).

Close intent is one enum owned by App. It owns no draft, transaction or native
resource. The existing worker remains authoritative through file-lock waits and
failure; results are collected before approved exit. Media is checked again at
final close. Abnormal process termination retains the existing atomic-file limit;
this change does not claim transactional rollback on process death.

## T1786 — native light appearance prototype and integration

The Linux startup adapter imports GTK's light palette, font, text scale and
animation preference into the independent Appearance contract. This machine reports
Mint-Y-Aqua, Ubuntu 10 pt, scale 1.0, background RGB(248,248,249), foreground
RGB(32,32,32), and accent RGB(31,158,222). The font bytes come from fontconfig.
The owner’s light preference is retained: a dark-only palette is rejected with
an explicit standard-light fallback. No desktop preference is changed.

| Appearance input | Existing egui-only setup | Implemented GTK adapter | KDE boundary |
| --- | --- | --- | --- |
| Window/view colors and accent | Fixed light colors | Native semantic colors; text contrast >=4.5:1 | KColorScheme provides separate role/state sets; dedicated adapter remains tracked |
| Font | Bundled default | Actual desktop font, point-size conversion | Requires native QFont/KDE font extraction |
| Text scaling | Fixed sizes | Desktop text scale via UI zoom, including explicitly sized labels | Requires native scale acceptance |
| Reduced animation | egui default | GTK animation preference | Requires native preference extraction |
| High contrast | Generic light palette | Imported light colors with contrast correction | Dedicated native high-contrast session remains unvalidated |
| Widget rendering/metrics | egui | Still egui; not GTK widgets | Still egui; no Qt-widget parity claim |
| Live appearance changes | None | Read once at GUI startup; explicit in General | Future bounded subscription outcome tracked |

Source contracts: [GTK color lookup](https://docs.gtk.org/gtk3/method.StyleContext.lookup_color.html),
[GTK default font](https://docs.gtk.org/gtk3/property.Settings.gtk-font-name.html),
and [KColorScheme roles/states](https://api.kde.org/kcolorscheme.html). Mapping native
values into egui is Blent's adapter design, not a toolkit guarantee of native rendering.

Runtime dependencies are optional `/usr/bin/python3`, PyGObject/GTK3 introspection
and fontconfig; unavailable probes retain explicit light defaults. Debian/Ubuntu:
`python3-gi gir1.2-gtk-3.0 fontconfig`. The script is embedded in the GUI binary.
No new Rust package/version is introduced: direct ab_glyph/libc usage reuses the
existing resolved versions. The current GTK snapshot is used at startup, not on
tray reactivation of an already-running window.

Lifetime: the existing bounded command supervisor owns each original Python and
fontconfig child/collector, including cancellation and unknown retirement. Its
configured capture budgets apply before the 8-KiB snapshot/4-KiB path validation.
One regular font is read with nonblocking open, capped at 8 MiB, and validated by
the same font parser family as egui before handoff. The context owns the font bytes
until context retirement. Snapshot strings are <=256 bytes; no listener, native
window or borrowed toolkit pointer escapes. The GTK window is destroyed even on
partial lookup failure. A FIFO fault regression exposed a blocking-open risk in
the draft adapter; it failed before nonblocking open and passes afterward.

Verification: the original palette/font regression failed before integration;
seven targeted Rust tests, 13 appearance-filtered tests, four Python tests and the
private native production-startup test pass. All 19 maintained Rust functions have
93.6–100% line coverage; all four Python probe functions have 100% line coverage
(coverage.py 7.15.2). Measured Rust/Python functions have complexity <=9.
[Evidence](artifacts/2026-11-22-ui-ux/t1786/). Dedicated KDE integration and live
appearance refresh are explicitly decomposed as T1794/T1795, rather than claimed
implemented by this GTK prototype. Native visual acceptance follows deployment.

## T1787 — field semantics and keyboard navigation

Encoder/FPS and camera image/sensor controls associate their visible labels with
accessible controls. Combo boxes expose the selected value; sliders retain their
numeric value and bounds. Disabled shadow/adaptive/exposure controls explain the
capability or prerequisite. Parent/child navigation publishes selected state.
No focus order, configuration transaction, capture or resource admission changes.

The original encoder association regression failed. Four semantic/keyboard tests
pass, including Tab from Encoder to Frame rate, camera names/values and disabled
reasons, sensor associations and parent/child selection. All 44 camera-filtered,
16 Overview, existing encoder/dropdown and sensor regressions pass. All 15
maintained methods have 100% measured line coverage; 263 measured functions have
complexity <=9. [Evidence](artifacts/2026-11-22-ui-ux/t1787/).

The adapters borrow frame-local widget responses and copy bounded field labels or
values into egui-owned accessibility nodes. No retained widget reference, native
subscription or duplicate configuration owner is created. Native Orca/AT-SPI
consumer qualification is separately visible as T1796; the emitted tree and
hosted keyboard checks do not establish that acceptance.

## T1797 — clean native idle fixture

The final suite exposed an idle fixture inheriting an unrelated dirty draft. Its
scheduled close correctly opened T1783's new prompt, so the old fixture could not
finish. It now explicitly starts clean. All four private native modes complete
with their original idle callback bounds and old-timer fault controls intact.
No production idle behavior changed. [Evidence](artifacts/2026-11-22-ui-ux/t1797/).

## T1784 — readable tablet guidance

Settings help uses Material bodySmall (12 sp) and onSurfaceVariant on the semantic
surface, replacing 11-sp low-contrast custom colors. Guidance scales with Android
font settings. No setting application, camera or display lifecycle changes.

The permanent 200%-font landscape regression failed on the original 11-sp help.
It now checks actual composed text size, >=4.5:1 contrast, scrolling visibility
and access to Apply on Android API 27 and 34. All 68 test executions in four
settings classes pass. All 46 SettingsSheet-emitted methods with JaCoCo line
counters are covered at 100%, including callbacks; no aggregate substitution or
counter exclusion was added. JaCoCo 0.8.15; measured Kotlin complexity <=5.
[Evidence](artifacts/2026-11-22-ui-ux/t1784/). Native tablet large-font/landscape
acceptance follows the final signed installation.

Theme values are borrowed during Compose rendering; existing composition owns
layout and text. No new state, listener, allocation owner or native resource is
introduced. The finite test font scale is 2.0; this does not claim every possible
screen/font combination fits without scrolling.

## T1800 — clean page fixtures

The clean-home responsive and About-navigation fixtures explicitly start from saved preferences;
it previously inherited an unrelated width edit and contradicted T1782's required
global dirty footer. Its no-Save and clipping assertions remain unchanged and pass
at all three sizes. All six About tests also pass. Dirty navigation has separate T1782 coverage.
[Evidence](artifacts/2026-11-22-ui-ux/t1800/). No production behavior changed.

## T1785 — explicit tablet video draft

The video section owns its adjacent **Apply video changes** action and pending
indicator. Unchanged video cannot be applied. **Done**, back/outside dismissal
and the sheet's dismiss gesture ask before discarding pending video edits; Keep
editing preserves them. Immediately applied app/display switches remain applied.
Only edited video fields are sent, persisted and resent on reconnect by the
existing settings session. Rejection and camera state behavior remain unchanged.

The sheet remains the sole draft owner. Compose owns the bounded two-field draft,
confirmation flag and callbacks; the remembered sheet transition reads current
pending state through rememberUpdatedState. No media owner, native resource or
listener is added. Discard clears only the draft; Apply clears it after the
existing event handoff. No duplicate settings persistence is introduced.

The named/adjacent action regression failed before implementation. Permanent
regressions cover unchanged rejection, immediate versus staged controls,
Keep editing/discard and the sheet dismiss gesture on API 27 and 34. Existing
settings-session tests retain rejection, reconnect and camera-state checks.
Verification evidence: [T1785](artifacts/2026-11-22-ui-ux/t1785/).

All 98 test executions across eight selected classes pass. All 64 emitted
production methods with JaCoCo line counters are covered at 100%; measured
Kotlin complexity is at most 7. Native tablet acceptance follows installation.


## T1805 — explicit service presentation capabilities

Connections service controls receive the existing independent capability contract
from their caller. Production capabilities and service ownership remain unchanged;
tests exercise unavailable, daemon-only and display-capable states without enabling
any unsupported backend. The permanent eight-case presentation test and private
start/stop action test pass. No native state, allocation or callback owner is added.
Final per-function coverage includes the formerly unmeasured capability branches.


## T1798 — final GUI formatting and verification

Authoritative rustfmt was applied to the maintained GUI source batch. The GUI
format check passes. The final GUI suite passes all 302 tests; the table-driven
capability fixture also passes after its complexity-only rewrite. All 94
selected maintained Rust production functions meet >=80% executable-line coverage
individually on the formatted source. Measured complexity remains <=9. The final
coverage includes the full suite and isolated service-action run.

Production GUI Clippy with dependency linting disabled passes under Rust 1.100.0.
The full strict gate remains obstructed by preexisting broker type complexity
(T1801) and a paused audio test initialization lint (T1804). Preexisting host format
drift remains T1799; no whole-repository formatting or Clippy pass is claimed.
[Final evidence](artifacts/2026-11-22-ui-ux/final/).


## T1806 — native acceptance follow-up

The native check caught refresh-rate help outside SettingsSheet still using 11sp and a low-contrast color. The extended permanent 200%-font contrast regression failed before the fix and passes using Material bodySmall/onSurfaceVariant. This stateless composable borrows existing settings and event callbacks; no resource lifetime changes.

All 6 emitted maintained methods with JaCoCo line counters have 100%
coverage; complexity <=9. The eight-class final run passes 100 test executions
on API 27/34. [Evidence](artifacts/2026-11-22-ui-ux/t1806/).
