# Host review fixes (2026-10-10)

## T831: apply coverage scope before parsing

Full inventory previously parsed the archived T790 C candidate and failed before
filtering out already-exempt documentation/benchmarks. Inventory now applies the
existing `coverage_source` policy first. No maintained-code exclusion or coverage
threshold changed. A permanent regression adds malformed exempt C/Rust fragments,
checks the maintained function remains inventoried, then proves malformed
maintained Rust still fails closed. All 69 coverage-tool tests pass; a full current
project snapshot succeeds. [Red/green logs](artifacts/2026-10-10-t831/).
