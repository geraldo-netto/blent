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
