# T922 typed display status

Typed display status represents native USB peers without requiring ADB serials. Linux regressions retain native-only status during ADB failure; Windows runtime evidence remains unavailable.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Native USB records no longer require an ADB serial. Existing ADB JSON shape remains
unchanged. Permanent regressions fail before the schema/GUI/diagnostic fixes and
pass afterward. Invalid-input tests cover ambiguous/missing identities, malformed
fingerprints, control characters, oversized serials, and slot/port boundaries.
Native-only consumers avoid ADB; mixed status survives ADB failure. Existing ADB
selection, model labels, forwarding/actual ports, Wi-Fi and slot publication remain
covered in their normal suites.

Validation: 272 common library tests plus its normal integration suite pass;
365 host library and 425 host binary tests pass (3 and 4 existing benchmark ignores);
all 101 GUI tests pass. All 21 scoped maintained production functions have at least
80% executable-line coverage. All 330 reviewed production/test functions have
cyclomatic complexity at most 9. Exact executed LLVM objects were exported to
avoid stale cached mappings under T880. Wider untouched diagnostic entry points
require their normal integration executable mappings and are not claimed covered
by this unit-suite collection.

Windows production callers cross-compile offline. Windows test checking exposed
the pre-existing missing connection panel in its App fixture (T926). Native Windows
status runtime/counters remain unvalidated under T902; Windows native USB remains
unsupported. T923 separately retains the pre-existing Linux status snapshot bounds
finding. Inventory includes the separate pending T855 fixture; that fixture remains
uncommitted and is outside this change.

```sh
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent-config
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent-config -p blent --bin blent --lib --lcov --output-path /tmp/blent-t922-host.lcov
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent-gui --bin blent-gui --lcov --output-path /tmp/blent-t922-gui.lcov
CARGO_NET_OFFLINE=true cargo check --offline --locked --target x86_64-pc-windows-gnu -p blent -p blent-gui
```

No downloads, installs, reloads, device-mode changes or physical tablet acceptance.

</details>
