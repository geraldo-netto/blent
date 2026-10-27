# T912 retained session listeners

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The permanent ephemeral-port regression failed first because Runtime reported
configured video port 0. It now verifies distinct nonzero, actually owned ports.
Session failure/lifecycle tests (16) and capture-contract tests pass offline.
All 16 scoped production functions meet individual 80% executable-line coverage.
Current-executable LLVM export avoids the known T880 reused-target mapping issue.

Retained endpoint fixtures verify video/control ordering, duplicate native ownership
through actual session retirement, final descriptor release, compatibility startup,
partial bind rollback before capture, and existing capture-start panic rollback.
Only local TCP sockets and synthetic capture/input adapters are used.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --test session_failure --test capture_contract --lcov --output-path /tmp/blent-t912.lcov
llvm-cov export --format=lcov --instr-profile=target/llvm-cov-target/blent.profdata --object target/llvm-cov-target/debug/deps/capture_contract-d0ba440204bce380 --object target/llvm-cov-target/debug/deps/session_failure-77704dfd80a32026 > /tmp/blent-t912-current.lcov
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t912-manifest.json --lcov /tmp/blent-t912-current.lcov --scope host/src/session_core.rs --scope host/src/session_network.rs --output /tmp/blent-t912-coverage.json
```

The network adapter uses portable standard TCP facilities; this Linux measurement
is not native Windows acceptance. No install, reload, downloads or physical USB
trial. Production host integration remains T908 and joined display/input T857.
