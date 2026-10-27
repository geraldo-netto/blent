# T925 joined display ownership

Joined display ownership retains registration, native work and listener capabilities until each original cleanup completes. Canceled startup cannot acquire replacement authority; production daemon and physical acceptance remain separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`registered_session` joins independent registration and native-work contracts.
Capacity values and read-only reservation validation live in a separate contract;
neither interface exposes concrete capture, UI, storage or OS types. Preflight
rejects stale, closing and wrong-peer reservations, invalid ports and retry bounds.
Binding revalidates after native startup. A cancelled startup returns a closing
owner retaining native work and original listener capabilities without acquiring
or revoking a replacement attachment lease.

An active exact registration publishes typed USB status. Decoder/control renewal
keeps capture configuration and geometry, rotates credentials and does not stop
native work. Whole-owner cancellation waits for registration and actual native
retirement. Composition must drop the retired owner and its listener capabilities
before releasing its capacity ticket. Timeout or logical close is not completion.

`RuntimeWork` retains extra runtimes through their supervisor and native completion;
primary bindings observe the daemon-owned runtime and never stop primary capture.
The Linux registration adapter duplicates retained real listeners and keeps their
original capabilities pinned through cleanup. The existing T911 real TLS fixture
is retained and also exercises this adapter bidirectionally for T925.

Two permanent draft regressions failed before their fixes: closing tickets admitted
startup, and a preflight cancelled during native startup constructed another owner
over replacement authority. Both now pass unchanged in the normal suite. Fourteen
T925 tests cover independent retirement order, bounded invalid inputs, stale
authority, renewal, native failure, drop, retained ports and shared-primary reuse.
All capacity tests and sixteen existing session-failure integrations pass.

All 39 scoped production functions meet individual 80% executable-line coverage;
all 106 reviewed production/test functions have cyclomatic complexity at most 9.
Native counters use the exact freshly executed test binary to avoid T880 stale
object mappings. Offline Windows production/test checking passes; native Windows
runtime validation remains T902. Production daemon composition remains T908 and
joined tablet display/touch acceptance remains T857, including postponed T855.

```sh
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --lib --lcov --output-path /tmp/blent-t925.lcov -- t925
CARGO_NET_OFFLINE=true cargo llvm-cov --offline --locked --no-clean -p blent --features accessory-transport --lib --lcov --output-path /tmp/blent-t925-slots.lcov -- display_slots::
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent --features accessory-transport --lib
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent --test session_failure
CARGO_NET_OFFLINE=true cargo check --offline --locked --target x86_64-pc-windows-gnu -p blent -p blent-gui --tests
```

No downloads, installation, live broker/device changes or physical acceptance.

</details>
