# T911 Linux media-registration client

The Linux registration client now sends Unregister after idle close and preserves local versus remote retirement evidence. Isolated socket/TLS tests do not establish tablet acceptance.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Fourteen permanent T911 tests and the 90-test Linux broker suite pass offline.
All 33 scoped production functions in registration_client and endpoints meet
individual 80% executable-line coverage. Source-matched current executable export
avoids known T880 stale cached-object mappings; both exports remain. Complexity:
58 production/test functions, none above 9 (see complexity.json).

The idle-close regression failed first: close woke receive but did not return to
send Unregister. The same permanent test now passes. Native fixtures cover exact
broker/TLS registration, duplex control payload, cancellation, invalid settings,
descriptor rejection/retention, failed IPC, sibling progress, repeated-close
absolute deadlines, panic with blocked runtime cleanup, and separate native/remote
retirement proof. White-box cleanup in failed-IPC fixtures is explicitly labelled;
production never synthesizes remote retirement after losing IPC.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --lib --lcov --output-path /tmp/blent-t911.lcov -- broker::linux
llvm-cov export --format=lcov --instr-profile=target/llvm-cov-target/blent.profdata --object target/llvm-cov-target/debug/deps/blent-592b3b54ab5a39c1 > /tmp/blent-t911-current.lcov
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t911-manifest.json --lcov /tmp/blent-t911-current.lcov --scope host/src/broker/linux/registration_client --scope host/src/broker/linux/endpoints.rs --output /tmp/blent-t911-coverage.json
```

The broader suite includes pending T855's local presentation/TLS regression;
its physical acceptance remains deferred, and that test is outside this commit.
Fixtures use isolated sockets and synthetic USB streams, not the tablet.
No download, install, mode change or live restart. T912/T908 still own session
listener publication and production host integration.

</details>
