# T1221 actionable offline parser replacement

The owner requested moving implementable blocked work to Open on2026-10-21.
T1221 moves because the earlier T1244 clarification accepts local cyclomatic
tools: repairing this source-inventory/parser gap does not require the exact
unavailable tree-sitter-rust0.24.2 package or a metered download. A supported
offline Rust AST frontend can replace the old Rust parser and update its current
metric/inventory callers together, without a compatibility fallback.

Cached `syn2.0.119` exposes async closure syntax and parses its `async` token in
`src/expr.rs`. Its full/visit features and proc-macro2 span support resolve with
all dependencies present locally: proc-macro2 1.0.107, quote1.0.47 and
unicode-ident1.0.26. Installed compiler is Rust1.98.1 and Cargo1.98.1. Cargo
metadata resolves first offline, then offline/locked, in a private network
namespace. The minimal readiness crate has an empty library and introduces no
functions or metric implementation. No package is installed or downloaded.

This proves dependency availability and inspects supported parser source, not
end-to-end integration or a passing replacement metric. The existing ABI14
grammar still fails on current OSK async closures, and the previously recorded
three normal source-inventory fixture errors remain unresolved. Preserve
independent scoring/boundary contracts, existing 9/10 and parser rejection
oracles, correct Rust syntax and full normal source scope. Add permanent async
closure, invalid syntax and source-span regressions with the actual replacement,
then pass normal metric/source-inventory suites without suppressions or skips.
The shared parser/inventory replacement is one cohesive medium-effort deliverable.

[readiness.json](readiness.json) records local versions, cached source hashes and
the resolved dependency closure. [Cargo.toml](Cargo.toml), [Cargo.lock](Cargo.lock)
and [lib.rs](lib.rs) retain the exact availability probe. Cached source is
borrowed read-only during inspection; Cargo owns only its private local lock and
metadata outputs. Both commands settle before inspection, with no retained
native process, callback or device handle. No physical device, running service,
EVDI implementation or ownership contract is changed by this classification.

Other blocked items retain actual prerequisites: T1211/T1131/T1128 require
explicit resumption of excluded EVDI work; T835 depends on T1211; T748 also lacks
its stock recovery/trusted-install prerequisites. T1090's snapshot/remove race
still needs a native conditional-removal contract or an explicit external
ownership/exclusivity decision. Current camera/audio callers still use that ADB
adapter; the no-compatibility policy alone does not authorize deleting an active
transport. Declined macOS/stylus dispositions remain unchanged.
