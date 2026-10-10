# T920 registered display renewal lifecycle

A portable per-display owner now retains registration and generation-scoped cleanup
through channel loss, backoff and cancellation. Native descriptors and capture
sessions remain outside the contract. T908 owns production daemon composition;
this is not physical tablet or USB rendering acceptance.

Permanent regressions failed against the draft before fixes:
- Late close revoked the replacement attachment.
- A stale owner retried over a newer attachment after native retirement.
- Replacement during the retry window was overwritten by unconditional renewal.
- Invalid active route counters admitted display.

Conditional renewal now compares and advances generation under one lock. Revocation
returns only authority for its exact unavailable generation. Stale detachment/drop
cannot change a replacement. One of four concurrent attempts can renew an observed
generation; the other three fail without mutation. Ordinary hot renewal preserves
geometry/presence, while explicit whole-display stop clears its own metadata.

Validation: all 364 library tests passed; three existing opt-in benchmarks remain
ignored. The 24 maintained production functions across registered_display.rs and
attachment/registered.rs each meet 80% executable-line coverage; all 64 scoped
production/test functions have cyclomatic complexity at most 9. Permanent bounded
tests cover invalid phases/counters, changed routes, retry boundaries, native
completion, entropy/startup failure, stale work and concurrency. Repeated active
polls do not wake capture. No resource-use or latency improvement is claimed.

Exact current test-binary mappings were exported to avoid T880 stale objects.
The global inventory includes the separate pending T855 fixture outside this diff
and outside the scoped coverage. No downloads, installs, reloads or live trials.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --lib --lcov --output-path /tmp/blent-t920.lcov
llvm-cov export --format=lcov --instr-profile=target/llvm-cov-target/blent.profdata --object target/llvm-cov-target/debug/deps/blent-276264e4cf99d16f > /tmp/blent-t920-current.lcov
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t920-manifest.json --lcov /tmp/blent-t920-current.lcov --scope host/src/registered_display.rs --scope host/src/attachment/registered.rs --output /tmp/blent-t920-report.json
```
