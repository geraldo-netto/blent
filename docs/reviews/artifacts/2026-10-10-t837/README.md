# T837 deterministic overflow fixture retirement

Denied-output producers now remain alive after their writes until the command
owner retires them, with a ten-second self-expiry. They cannot reach normal
exit-time LLVM flushing while the parent intentionally kills them for overflow.
Successful output fixtures and all exact-limit/overflow assertions are unchanged.

The permanent T837 readiness/lifetime regression failed before this change and
passes afterward. The normal common library, command lifetime, command output
and owned-stream suites pass: 229 tests. Fresh profiles merge under LLVM's
normal strict behavior; no invalid-file bypass or coverage exclusion is used.
The resulting strict counters also pass all 12 T836 production methods at
individual >=80% line coverage and cyclomatic complexity <=9. No production
source changes are needed for this fixture fix.

Offline validation (after clearing old raw profiles):

```sh
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/tmp/blent-t836-target cargo llvm-cov \
  --offline --locked -p blent-config --lib --test command_lifetime \
  --test command_output --test owned_stream --lcov \
  --output-path /tmp/blent-t837-strict.lcov
```

Retained logs include the failing pre-fix regression, full strict green run,
LCOV counters, scoped method report and source hashes. No downloads or live
service changes. Native Windows execution remains tracked under T737.
