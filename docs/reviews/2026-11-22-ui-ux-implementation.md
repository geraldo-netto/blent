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
