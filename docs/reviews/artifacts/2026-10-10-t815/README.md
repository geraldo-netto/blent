# T815 bounded command output

No downloads, production reloads, physical-device operations or dependency-source
changes. Five retained red logs cover unlimited output, overflow hidden behind
blocked stdin, signalling after direct-child reap, synchronous held collection,
and an unrepresentable command deadline. `collection-red` restores the former
collection-on-caller behavior through the new injectable reader boundary; the
remaining logs exercise the code in place before the corresponding corrections.
All regressions remain in normal suites, including exact/over-limit and combined
stream boundaries, inherited writer handles, held reads, interrupted reads,
late child exit after an idle probe, malformed settings, panics and cancellation.

Fresh source-attested default collection passes 977 tests with seven unchanged
existing ignores. All 47 maintained production functions in the changed command,
collector, limits, pipe, shared thread-retirement and control-worker modules meet
individual 80% executable-line coverage and complexity <=9. Source bytes stayed
unchanged during collection. Also passed: 128 no-default common tests, four host
architecture guards, and offline x86_64-pc-windows-gnu common cross-compilation.
Native Windows pipe/job execution and counters remain T737.

Collection: `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/tmp/blent-t815-target
cargo llvm-cov --offline --locked -p blent-config -p blent --lib --bin blent
--test command_output --test command_lifetime --test owned_stream
--test portable_session --test session_failure --test shutdown_deadline --no-report`.
The cumulative report was generated with `cargo llvm-cov report --offline --locked
-p blent-config -p blent --lcov`. Source hashes and function details are retained
beside the compressed LCOV and logs. No throughput or end-to-end latency gain is
claimed; the tested improvement is bounded output storage and control work.
