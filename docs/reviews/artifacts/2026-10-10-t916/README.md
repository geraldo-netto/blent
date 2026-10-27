# T916 registered attachment renewal

Attachment renewal now binds credentials and generation transitions atomically. Stale, absent or exhausted transitions cannot restore authority or cross another handoff.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

All 28 scoped maintained functions meet individual 80% executable-line coverage.
Eleven final attachment tests pass. Broader unchanged production code passed the
host library suite (344 passed, three existing ignored tests), capture contracts
(four) and session lifecycle (16). A final permanent pending-socket test was then
added and the attachment suite rerun; coverage includes that test and the broader
suite's counters. No tests were removed, skipped or weakened.

Three draft behavioral regressions failed before their fixes and remain permanent:
concurrent lease capture crossing another credential/identity handoff, absence
restoring tokenless authentication while native ownership retired, and native
detach/legacy fallback wrapping the generation counter. Other fixtures cover all
256 credential byte values across 2,048 bounded concurrent handoffs, stale/late
admission, same-peer geometry and presence, fresh-peer reset, drop/revoke, pending
socket I/O, legacy fallback policies and closed/exhausted sessions.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --lib --test session_failure --test capture_contract --lcov --output-path /tmp/blent-t916.lcov
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --lib --lcov --output-path /tmp/blent-t916.lcov -- attachment::
llvm-cov export --format=lcov --instr-profile=target/llvm-cov-target/blent.profdata --object target/llvm-cov-target/debug/deps/blent-276264e4cf99d16f --object target/llvm-cov-target/debug/deps/capture_contract-d0ba440204bce380 --object target/llvm-cov-target/debug/deps/session_failure-77704dfd80a32026 > /tmp/blent-t916-current.lcov
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t916-manifest.json --lcov /tmp/blent-t916-current.lcov --scope host/src/attachment.rs --scope host/src/attachment/auth.rs --scope host/src/attachment/registered --output /tmp/blent-t916-coverage.json
```

Current-object export avoids known T880 stale mapping aggregation. Ordinary legacy
counter exhaustion remains separately tracked under T865; new native transitions
fail closed. No USB/physical support claim, downloads, install or live restart.

</details>
