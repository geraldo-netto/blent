# T910 portable registration lifecycle

Nine permanent tests pass, including all 25 bounded status pairs, malformed and
extreme request/route counters, cancellation before/after exposure, failed
admission, stale/foreign replies and exact remote retirement. All 14 maintained
production functions meet individual 80% executable-line coverage. Production and
test complexity: 28 functions, maximum 8, zero above 9.

Commands (cached dependencies; no downloads):

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent-config --lib --lcov --output-path /tmp/blent-t910.lcov -- connection::registration_client
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t910-manifest.json --lcov /tmp/blent-t910.lcov --scope common/src/connection/registration_client.rs --output /tmp/blent-t910-coverage.json
```

This is a portable protocol model, not native USB or rendered-frame acceptance.
Linux ownership/worker adaptation follows in T911; host session integration in
T908. Physical tablet pairing remains owner-deferred T855.
