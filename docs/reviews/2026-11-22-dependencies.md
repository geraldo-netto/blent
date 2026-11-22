# Dependency cleanup — 2026-11-22

Different versions in bundle notices identify actual Cargo dependency chains.
They must remain listed until their consumers are rebuilt without them. SHA-1
and SHA-2 are different algorithms, not duplicate versions of one library.

## T1730: one D-Bus stack

Host, tray and accessibility now share zbus 5.16 and its async-io backend.
Enabling zbus's Tokio feature would require a reactor on AccessKit's dedicated
worker. Upstream supports runtime-independent async-io use:
<https://docs.rs/zbus/5.16.0/zbus/#compatibility-with-async-runtimes>.

The production adapters compile unchanged. Retained fixtures use the current
message builder API and async-io timers in their D-Bus service callbacks; their
previous Tokio timers panic when called on the shared executor. Reply identity,
foreign sender, cancellation, retained native debt and original retirement
assertions are unchanged. No compatibility wrapper or fallback was added.

95 affected activity/native-command/tray tests pass. New permanent T1730 test
round-trips a private D-Bus connection on a thread with no Tokio reactor and
closes it before joining; it passes. Local Rust syntax complexity checker
(syn 2.0.119) measures every function in the four changed fixture files, all <=9.
No production function body or resource contract changed. Exact native settlement
remains the existing adapter/connection owner's responsibility.

Cargo.lock drops nine package versions and seven duplicated package names:
zbus, zbus_macros, zbus_names, zvariant, zvariant_derive, zvariant_utils and nix.
Lockfile totals cover all recorded targets/features, not Linux runtime code:
551 -> 542 package versions; 43 -> 36 names with multiple versions. Remaining
release-graph and installed-notice evidence belongs to T1734.
