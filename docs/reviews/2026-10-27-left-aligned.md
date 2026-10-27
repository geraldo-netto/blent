# Left-aligned GUI refinements — 2026-10-27

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
