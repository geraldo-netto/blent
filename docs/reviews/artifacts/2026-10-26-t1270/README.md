# T1270: route-specific connection guidance

A missing ADB installation no longer marks the Linux direct USB display path
as needing setup. Display dependency/device checks still apply. The optional
ADB notice explicitly covers ADB-selected camera/audio operations. Disconnected
help offers paired direct USB without debugging and labels ADB debugging/
authorization instructions as applying to that route only. Windows preview and
unsupported-backend labels remain unchanged. README introduction, quick install,
comparison, FAQ and roadmap now distinguish the implemented routes as well.

Three permanent rendered-widget regressions fail against the prior production
code for incorrect mandatory ADB setup, missing debugging-free pairing guidance,
and missing route-specific qualification alongside genuine display prerequisites.
The same tests pass after the fix. A fourth test exhausts all four supplied
route-capability boolean combinations, including no supported route. Existing
private setup/action tests still check both package notices and now require the
optional-ADB qualification; every original lifecycle/action oracle remains.

Normal GUI unit suite: 120 passed. Native LLVM coverage suite: 120 passed.
All four changed production functions independently meet the 80% executable-line
gate: show_missing_packages 100%, show_setup 84.85%, show_status 81.08%, and the
new route presenter 100%. This is a changed-function scope, not a whole-project
coverage claim. Raw LCOV, source-bound report and logs are retained. Installed
cargo-llvm-cov 0.8.7/rustc 1.98.1/LLVM 22.1.8 ran offline/locked using the cached
SDK runner. Locked syn 2.0.119 measures 181 functions in the four changed Rust
production/test files; none exceeds complexity 9.

The route presenter is stateless and borrows only the current egui UI and two
booleans (their entire finite input domain is tested). Text is static and the UI
owns rendered output for the frame; no native handle, callback, queue or worker
is created. Existing setup commands and client cleanup remain with their current
owners. Native Windows execution is not inferred from the injected unavailable
capability cases; no native adapter changed in this item.
