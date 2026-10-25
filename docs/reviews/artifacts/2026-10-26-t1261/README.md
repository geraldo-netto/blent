# T1261: enforce ABI checks for every shipped binary

The independent seven-binary inventory and six USB invalid-ABI cases failed
against the original gate. All five test methods pass after including both USB
executables and updating the existing invocation-count assertion. The ordinary
host tooling target already invokes this unittest module.

`coverage.json` measures each production function with installed coverage.py;
`complexity.json` uses the project checker (Python ast via Python 3.12.3).
Both functions have 100% executable-line coverage; every scored function <=9.
The verifier borrows finite command output for one synchronous invocation;
this change expands a fixed seven-element inventory and introduces no retained
resources. Temporary test directories and mocks remain context-owned.
