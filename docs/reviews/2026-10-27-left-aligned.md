# Left-aligned GUI refinements — 2026-10-27

The owner replaced centered page content with stable left alignment and made About use one page scroll area. Follow-up geometry tests fixed centered notice text and reflowed only the displayed MIT prose while preserving copied bytes. Linux checks passed; these decisions supersede the earlier responsive-layout report, with Windows validation still separate.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

## T1292 — Stable left edge

Owner explicitly superseded T1289/T1290 centering. The page wrapper now keeps
its available left edge while retaining bounded form/card widths and adaptive
resource columns. The footer shares that same left edge. Existing permanent
resize regressions retain column counts, width limits, clipping and action
checks; centering expectations are replaced with exact left-position checks
under the new contract, including compact/desktop navigation transitions.

The updated regression failed before the fix (Settings x=40 rather than x=8
at width 1040). All 164 GUI tests plus native startup/accessibility pass after
it. content has 100% individual executable-line coverage, and every function
in changed files measures <=9. Tools: cargo-llvm-cov 0.8.7, LLVM 22.1.8, Rust
1.98.1; repository locked syn 2.0.119 complexity metric. Evidence is retained
under /backups/disk2/blent-work/2026-10-27-left-aligned.

Only a frame-local rectangle calculation changed. The synchronous UI borrow,
finite viewport bounds, static data and native resource owners are unchanged.
No retained allocations, callbacks or threads were added. Native Windows
visual verification remains tracked under T1271.

## T1293 — Expanding About content with one page scroll owner

Owner superseded the bounded About column and nested notice viewport. About
now takes the available page width; its selector follows that width. The full
notice became a selectable, wrapping label in page flow. Native acceptance
subsequently exposed centered placement of its text block; T1294 below corrects
that regression.
Only the outer page scrolls when the selected content exceeds the viewport;
long legal texts are neither truncated nor promised to fit every screen.
The selector popup retains its own scrolling for the document list.

Both permanent regressions failed before the change: selector bounds remained
520 points wide and the notice clip began inside the page. The accessibility
geometry test checks selector bounds through 380/1040/1920/3840-point resizes;
the existing reader regression now enforces the new single-viewport contract
and full visible MIT notice when space permits. Existing selection/copy,
keyboard and compact wrapping tests remain intact.

All 165 GUI tests and native startup/accessibility pass. maximum_width,
show_window, document_viewer and notice_text each measure 100% executable-line
coverage. All changed functions/tests measure complexity <=9 using the same
tools as T1292. Evidence: t1293-red.log, t1293-coverage.log,
t1293-metrics.json and t1293-complexity.json in the local artifact directory.

Static notice text remains borrowed for the current egui frame; copying still
creates only the explicit clipboard result. The page owns scroll state; the
removed nested viewport retains no separate state owner. No native allocation,
worker, device handle or callback ownership changed. Native Windows visual
acceptance remains T1271. Earlier responsive review results are historical;
this section records the current owner-approved layout contract.

## T1294 — Native MIT placement and paragraph reflow

Native acceptance and the owner identified two visible problems after T1293:
egui::Ui::add_sized centers the text block through its child layout even with
Label::halign(Min), and LICENSE hard-wraps prose around 75 characters. A new
permanent geometry regression failed with the notice at x=1611.3 in a 3840-point
window. It now checks left alignment, viewport bounds, wide prose use, paragraph
count and exact token preservation at four widths from 380 to 3840 points.

The label uses normal left-aligned flow. Only the known top-level MIT notice
has its prose lines joined inside paragraphs for display. The copy action still
uses original bytes; all other documents retain their original formatting and
borrowed storage, checked exhaustively over the embedded document table.

All 167 GUI tests and native startup/accessibility pass. notice_text and
notice_display_text have 100% individual executable-line coverage. Changed code
and tests measure complexity <=9. Artifacts: t1294-red.log,
t1294-coverage.log, t1294-metrics.json and t1294-complexity.json. The display-only
MIT string is bounded by the embedded source and owned for the synchronous
frame; egui owns resulting text layout. Other strings retain static lifetime.
No native resources, asynchronous borrows or retained cleanup obligations were
introduced. T1271 retains unavailable native Windows acceptance.

</details>
