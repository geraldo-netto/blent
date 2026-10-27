# Responsive GUI fixes — 2026-10-27

## T1289 — Resource workspace

The native 3840x2088 capture showed two 1800px columns. The permanent
same-context resize regression first failed at 3840 points (two columns instead
of four). The workspace now centers within 1960 logical points and lays out its
four resources in one, two or four complete columns. Compact layouts retain
wrapping; tablet selection and resource action ownership are unchanged.

All 162 GUI unit tests and the native startup/accessibility test pass offline.
Per-function executable-line coverage: show_window 100%, content 100%,
resource_columns 100%, show_resources 88.2%. The repository locked syn 2.0.119
complexity metric measures changed files/functions and the regression at <=9.
Counters: cargo-llvm-cov 0.8.7, LLVM 22.1.8, Rust 1.98.1.

The layout owns only frame-local dimensions and borrows egui's UI for the
synchronous callback. It retains no pointers, native allocations, workers or
callbacks after the frame. Viewport width is supplied by egui; one/four columns
bound the finite four-resource collection. Native Windows visual validation
remains tracked in T1271. Artifacts: /backups/disk2/blent-work/2026-10-27-responsive
(t1289-red.log, t1289-green.log, t1289-coverage.log, t1289-metrics.json).
