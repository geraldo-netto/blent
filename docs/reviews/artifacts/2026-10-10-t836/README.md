# T836 checked retirement grace

The permanent extreme-grace regression first panicked at unchecked Instant
addition. It now checks explicit InvalidGrace, bounded return, actual process
reaping and final dependent-lease release. Valid zero/nanosecond/millisecond/large
representable grace and cancellation-before-poll remain tested. Existing T814
held-native retirement also now exercises both accepted-but-ineffective signals
and EPERM in isolated disposable processes.

228 common library/lifetime/output/owned-stream tests pass, as do 13 host
session-failure/shutdown tests and offline Windows GNU cross-compilation. All
12 changed production methods meet individual 80% line coverage and complexity
<=9. Common command exports have no production-function change.

Strict profile merging exposed T837: the intentional output-overflow fixture can
be killed during exit-time LLVM profile flushing. Three truncated fixture files
and the strict failure log are retained. LLVM's supported `--failure-mode all`
merged the complete profiles for this scoped report; no production function or
test was excluded, and every maintained method has measured passing counters.
The fixture-collection race remains explicitly tracked for a separate fix. The
new retirement fixture also waits for its own child readiness before adversarial
retirement, so the native operation has demonstrably started.

Collection: `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/tmp/blent-t836-target cargo
llvm-cov --offline --locked -p blent-config --lib --test command_lifetime
--test command_output --test owned_stream --no-report`. Profiles were cleared
before the final run. Report: `cargo llvm-cov report --offline --locked
-p blent-config --failure-mode all --lcov`.

No downloads, production reloads or dependency changes. Native Windows runtime
and counters remain T737. Invalid grace requests do not claim native completion:
Drop signals and transfers ownership to the existing bounded retirement pool.
