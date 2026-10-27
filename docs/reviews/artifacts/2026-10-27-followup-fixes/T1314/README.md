# T1314: distinct native discovery coverage identities

The permanent `test_t1314_fixture_and_native_counters_remain_distinct` regression
failed before the fixture rename: GCC emitted two note files for three C sources.
After the rename, it passes and exports reports for all three exact source paths.
All five discovery tests pass; the full USB collector subsequently measured all
38 production functions at >=80% (T1313 evidence).

Local `scripts/complexity` Python AST measurement, Python 3.12.3:
`compile_arguments` 1, `reported_sources` 2, `setUpClass` 1,
`test_t1314_fixture_and_native_counters_remain_distinct` 2.

The class owns its temporary executable until class cleanup. The regression owns
its separate temporary binary and counters; synchronous child exit precedes
counter reads and directory cleanup, including failure cleanup. Gzip handles use
context managers. Inputs are three fixed repository C sources and bounded fixture
mode 0/capacity 64. Mock libusb only; no physical USB device is opened. No production
allocation or native resource lifetime changed. The C fixture contents are unchanged.
