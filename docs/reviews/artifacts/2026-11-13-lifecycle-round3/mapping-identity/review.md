# T1445 — stale server registration satisfies a new input creation

Confirmed private-protocol counterexample, medium severity/medium effort. No live Xorg, KWin, tablet or real uinput device was accessed.

`DeviceIdentity::for_instance` supplies the same name/product for successive payloads (`host/src/input/linux.rs`). `x11_list_entry` accepts that name; `map_x11_list` counts successful matching kinds without tying the server registration to the current constructor (`host/src/input/mapping.rs:385`). Once the old registration satisfies the expected count, `map_x11_devices` returns (`:265–276`). The follower then waits for tablet/mode/card changes (`host/src/input/linux.rs:1125–1137`), not native registration.

The fixture executes the actual backend with private command programs, an empty private sysfs view, and the existing marker 497 uinput interception checked before construction. It models asynchronous X server registration without claiming a physical compositor trace:

1. First payload maps server ID 41.
2. Detach destroys that kernel payload. Reattach creates a second payload while the server still lists the old same-name ID 41.
3. The mapper successfully maps ID 41 again and returns; native counts are two creations/one destruction.
4. Publish server ID 42 without a tablet/mode/card event. Commands remain unchanged during a 600 ms observation window. The source's completed mapping branch explains the missing retry; the window alone is not the finding.
5. A card event wakes the mapper, which now maps ID 42. This is the positive reachability control.
6. Final detach retires both fake payloads; the original command PID is reaped and backend completion settles.

`probe.log` contains the command trace and assertions. `provenance.json` identifies the frozen T1433 library used for initial confirmation; `current-draft.log` confirms the same counterexample against the T1439 payload/T1444 acknowledged-mutation draft. The relevant identity matching/early-return behavior is unchanged by T1439's retention draft. `complexity.json` records 20 retained probe/fixture functions, maximum 6, using the repository's Rust syn/C tree-sitter checker. No binaries, profiles or absolute dependency manifests are retained here.

Reproduce by compiling `probe.rs` as a Rust test with this repository's blent, blent_config, tokio, tempfile, libc and futures_util dependencies and `CARGO_MANIFEST_DIR` set to the absolute `host` directory. Run `--nocapture`; the fixture recursively runs the exact test with private environment/interposition. The exact local rustc argument vector and binary live in `/tmp/blent-mapping-identity` and are intentionally excluded from the repository.

Required permanent regression: reject a prior creation's same-kind registration, continue bounded registration discovery, map the actual new creation when it appears without unrelated state changes, and retain normal first attachment, same-payload remap, duplicate IDs, cancellation, exhaustion and eventual cleanup controls. Completion requires a creation-bound native identity with explicit bounded non-reuse/restart assumptions, preserving instance/product range checks and T1439/T1444 exact outcome ownership. This is separate from the original-command-retirement fix.

## Accepted implementation scope

The parent scoped T1445 to the confirmed X11 backend. Preserve KWin names and configuration behavior; no KWin registration race is established by this fixture. X11 creation names will retain fixed instance/product IDs and use a bounded creation sequence plus process namespace, with explicit entropy/exhaustion handling and across-restart uniqueness assumptions. Cursor preparation must receive the same exact name as the constructor and mapper.

Primary KDE sources explain the backend boundary: [connection.cpp](https://raw.githubusercontent.com/KDE/kwin/master/src/backends/libinput/connection.cpp) selects preference groups using vendor/product/name, and [device.cpp](https://raw.githubusercontent.com/KDE/kwin/master/src/backends/libinput/device.cpp) persists the chosen output. Per-creation KWin names would add distinct persistent groups. Source retrieval hashes are retained in `kwin-naming-design.json`; external implementation source is not copied into this repository.
