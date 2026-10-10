# T919 actual session retirement before slot reuse

Permanent isolated-process regression failed before the fix: the monitor returned
its retired slot after the supervisor's bounded logical stop, while a native
capture worker still ran. The same fixture now holds the slot, lets a sibling
retire, releases its native worker and only then observes reusable capacity.
The fixture uses actual spawn_blocking work and the production supervisor; short
stop/reap budgets are confined to its child process. Its subprocess wait is bounded.

All 43 monitor tests pass. All three maintained functions in retirement.rs meet
individual 80% executable-line coverage. Production/test complexity is at most 9.
The daemon's existing five-second shutdown bound remains unchanged. No device
mode change, install, reload, external dependency patch or downloads.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --bin blent --lcov --output-path /tmp/blent-t919.lcov -- monitor::
llvm-cov export --format=lcov --instr-profile=target/llvm-cov-target/blent.profdata --object target/llvm-cov-target/debug/deps/blent-3d154ee040abf851 > /tmp/blent-t919-current.lcov
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t919-manifest.json --lcov /tmp/blent-t919-current.lcov --scope host/src/monitor/retirement.rs --output /tmp/blent-t919-coverage.json
```
