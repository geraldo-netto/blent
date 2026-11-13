# T1379 — remove the Linux CLI compatibility alias

The daemon imports the independent `crate::cli` contract directly. The alias
file and Linux module export are removed. Exact obsolete-path search across
common/host/GUI Rust sources has zero matches. `cargo test --locked -p
blent-config --lib` passed 430/430 cases. No function body, native ownership or
runtime behavior changed; existing regressions remain, without artificial new
tests or a new executable-coverage claim.
