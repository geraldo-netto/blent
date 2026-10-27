# T921 reject ambiguous certificate-to-link registration

Registration now rejects ambiguous live links sharing one certificate before consuming endpoints. An unrelated certificate and the remaining original link still work after ambiguity clears.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Permanent regression establishes two native TLS links at distinct synthetic USB
locations using the same certificate/private key. Before the fix, register returned
success and sent its offer to the first link. It now rejects before consuming any
registration owner or endpoint claim. An unrelated third certificate still registers;
after one duplicate loses authority, the remaining original link registers again.
The fixture shuts down actual native workers even on the red-test path.

All 91 Linux broker tests pass, including existing registration, descriptor lifetime,
revocation and shutdown fixtures. All nine production functions in server/routes.rs
meet individual 80% executable-line coverage. All 24 scoped production/test functions
have cyclomatic complexity at most 9. TLS fixtures select approval by the requested
connection rather than whichever link appears first.

Exact current executable mappings were exported to avoid T880 stale cache mappings.
The tested TLS fixture file and global inventory include the separate pending T855
presentation trial test. Only the ambiguity module declaration and exact-connection
fixture selection changes belong to this commit; T855 remains deferred.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --lib --lcov --output-path /tmp/blent-t921.lcov -- broker::linux::
llvm-cov export --format=lcov --instr-profile=target/llvm-cov-target/blent.profdata --object target/llvm-cov-target/debug/deps/blent-592b3b54ab5a39c1 > /tmp/blent-t921-current.lcov
PYTHONPATH=/tmp/blent-python-offline:scripts/coverage python3 scripts/coverage/report.py check --manifest /tmp/blent-t921-manifest.json --lcov /tmp/blent-t921-current.lcov --scope host/src/broker/linux/server/routes.rs --output /tmp/blent-t921-report.json
```

No downloads, installs, device mode changes, reloads or physical tablet acceptance.

</details>
