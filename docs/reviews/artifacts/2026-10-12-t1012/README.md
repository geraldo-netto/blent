# T1012: restore the workspace formatting gate

`cargo fmt --all -- --check` fails before the change and passes afterward.
`cargo fmt --all` changes only layout and declaration ordering in nine files,
including the module declaration added by T1013. The reviewed diff preserves
behavior and every permanent regression; no artificial behavioral test is added
for formatting. `cargo check --offline --locked -p blent -p blent-config --features
accessory-transport` passes. The whole-project complexity gate still passes
22,208 functions with none above 9.

Native T1013 measurements remain historical evidence for the exact source hashes
in that commit; no unchanged-byte coverage claim is made after formatting.
The next implementation collects fresh counters for its maintained functions.
No dependencies were downloaded, platform validation inferred, or deferred work
reopened. Compressed red/green format, compile and complexity logs are retained.
