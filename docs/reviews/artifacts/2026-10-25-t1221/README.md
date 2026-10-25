# T1221 Rust source frontend replacement

The complexity, application coverage, platform and mutation inventories now use
the locked `syn` 2.0.119 frontend through a versioned source-record contract.
The obsolete Rust grammar dependency and all its callers are removed together.
Current Rust source, including the OSK async closures, is unchanged. Independent
metric rules, native coverage attribution, test filtering and include reachability
remain explicit. Macro contents remain opaque; no macro-expansion claim is made.

Permanent T1221 regressions first failed against the old parser for async closures
and the actual OSK source. The same tests pass after replacement. They cover
bounded 2–13 scores (including 9/10), invalid syntax, Unicode identifiers and UTF-8
spans, nesting, test boundaries, unknown guards and opaque macro contents.
A further prefix-span regression failed on the draft frontend before correcting
`syn::parse_file`'s BOM/shebang offset removal, and passes with original byte
offsets restored. Source files are not rewritten.

The normal suites pass without exclusions or skips: 36 metric tests, 106 coverage
report/inventory tests and two mutation-scope tests. The normal application source
snapshot completes. All 96 functions in changed source/test files have individual
local scores at most 9; the whole-project gate measures 28,041 functions, none
above 9. Rust formatting and offline locked Clippy with warnings denied pass.
An additional comparison found identical metric rows and production-function
boundaries for all 832 Rust files accepted by the old parser; its sole parse
failure was the unchanged OSK file. The new frontend also accepts that file.

`results.json` records tool versions, parser scope, source hashes and inventory
counts. Compressed logs retain failing-before/passing-after results and every
whole-project function score. C 0.24.2 was compiled from cached Cargo source for
the test environment; the cached Shell ABI14 and Kotlin PSI 1.9.20 parsers were
reused. No tool or dependency transfer/install, physical-device action, module
replacement or push was performed. This is source tooling validation, not new
native application coverage or Linux/Windows/Android acceptance.

The helper owns one input/AST/output per synchronous process; normal completion
and parse rejection reap it before any result is consumed. Rust drops owned
allocations; it exports copied records, without native pointers or callbacks.
Python's parsed-source cache is bounded to 32 entries, and standalone platform
predicate caching to 256 entries. There is no device/native-resource ownership
change. The containing lifetime and source-size bounds are documented in the
[frontend contract](../../../../scripts/rust-source/README.md). This nonessential
development tool is exempt from application per-function coverage, with its
permanent regressions retained in the normal suites.
