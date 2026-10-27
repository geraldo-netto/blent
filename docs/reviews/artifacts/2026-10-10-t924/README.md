# T924 bounded selected-USB discovery

Selected-USB discovery retains one client through retirement and rejects ambiguous new peers before preemption. Failed polls cannot slide retry deadlines or revive old snapshots.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The portable source owns one nonblocking control client and retains it through
actual native retirement. Absolute positive retry deadlines are capped at 60 s;
failed polls do not slide them, and exhausted clocks cannot spin replacement work.
Only Ready/Busy complete snapshots produce authenticated candidates. Duplicate
certificates exclude all new candidates for that peer before capacity preemption.
Existing registrations keep their exact authenticated link when a duplicate
appears. Connection and ticket changes or broker lifetime replacement invalidate
that ownership, even when persisted identity and counters repeat.

Candidates retain compact opaque lifetime tokens rather than obsolete device/trust
snapshots. A shared contract regression proves an old snapshot is released while
its token remains alive. Candidate membership is discovery intent, never media
admission. T920 still requires exact Active registration; T925/T908 join sessions.

Two permanent draft regressions first failed: failed-client diagnostics disappeared
on retirement, and completed native clients retained Ready status. Failures now
remain visible during retry, bounded to 512 characters; retained terminal status
has no old snapshot or command outcome, and unexpected completion becomes Closed.
These tests remain in the normal host library suite.

Validation: all 13 T924 host tests, all 3 native launch tests (including the moved,
unchanged T894 path assertions), all 13 shared control-view tests and all 11 GUI
connection-settings tests pass. All 35 scoped production functions meet individual
80% executable-line coverage. All 98 reviewed production/test functions have
cyclomatic complexity at most 9. Exact current test mappings were exported to
avoid T880. Windows production and normal test targets cross-check offline;
this does not establish native Windows USB support or runtime counters.

Linux launch-path resolution is shared by daemon discovery and existing GUI
composition. It resolves configured absolute roots and sibling binaries without
directory scanning, USB access or token reads. Native tests exercise real worker
startup/cancellation/retirement in private temporary directories.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --lib --lcov --output-path /tmp/blent-t924-host.lcov -- t924
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --lib --lcov --output-path /tmp/blent-t924-launch.lcov -- broker::linux::launch::
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent-config --lib --lcov --output-path /tmp/blent-t924-common.lcov -- connection::control_view::
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent-gui --bin blent-gui --lcov --output-path /tmp/blent-t924-gui.lcov -- connection_settings::
CARGO_NET_OFFLINE=true cargo check --offline --locked --target x86_64-pc-windows-gnu -p blent -p blent-gui --tests
```

No downloads, installs, reloads, live broker/device changes or physical acceptance.
The separate pending T855 TLS fixture remains uncommitted.

</details>
