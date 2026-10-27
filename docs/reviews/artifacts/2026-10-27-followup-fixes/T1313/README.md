# T1313: collect, retain and merge native USB coverage

Three permanent CI workflow regressions failed before wiring the collector,
upload and combined-gate input. The collector suite regression separately failed
before including the existing discovery and selection suites. All six workflow
tests and three collector tests pass after the changes; five compiler tests pass.

The real instrumented native suite passes all 34 tests. GCC/gcov 13.3.0 measure
all 38 maintained helper functions at >=80%; none is missing or below threshold.
`report.json` is the collector output. `combined-usb-slice.json` independently
loads its gcov files through the combined gate's `native_lines` and `collect`
functions and passes the same 38-function USB scope. Source hashes are retained
in `manifest.json`. This validates the USB slice, not the entire cross-platform
CI workflow. No remote CI run or physical USB acceptance is claimed.

Local `scripts/complexity` Python AST measurement, Python 3.12.3:
changed `capture_check.suite` 3; four added regression methods 1 each.

The suite builder retains only a local bounded repository file list and returns
owned argument strings to the existing synchronous subprocess collector. Native
fixture resources and output cleanup remain with the existing test owners.
Coverage artifacts intentionally remain in the selected evidence directory.
No production C code or native lifecycle changed. T1314 separately fixes the
fixture filename collision exposed by including the discovery suite.
