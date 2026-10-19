# T1127 Rust formatting

The authoritative wrapper failed its first stage, Cargo workspace formatting,
on six existing Rust files (eight diff locations). Formatting those files fixes
that stage; `cargo fmt --all -- --check` now passes. Changes are module ordering,
wrapping and one formatter-required match-arm block around a string expression.
Existing tests and executable expressions are preserved; no artificial tests
or production coverage campaign is added for formatting alone.

Running the full wrapper also exposed drift in its Linux-supervisor include
stage. Those twelve capture/monitor/supervisor fixture paths were restored to
exact HEAD bytes and left untouched under the owner's EVDI-work exclusion.
T1128 retains that explicit deferred scope, listed in skipped-paths.txt. A whole
wrapper pass is not claimed. No Windows source is changed. The pinned repository
source complexity audit remains <=9 for every function in the six formatted files;
actual native SonarQube evidence remains unavailable under T1019.
