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

## T1735: remove unused direct GUI edges

GUI sources have no serde caller; the only direct TOML use is the T1552 fixture.
Remove the unused serde declaration and move TOML to dev-dependencies. Four T1552
configuration-observation tests and the production release GUI build pass.
Transitive serde/TOML uses remain; this cleans ownership of direct dependencies
without claiming fewer resolved package versions. No function body changes.

## T1737: restore the retained capture integration test

The all-target compile exposed an existing stale caller in
`host/tests/capture_contract.rs`: `subscribe()` omitted the now-required optional
liveness capability (E0061). The fixture now passes None explicitly and retains
all pointer identity, generation, rejection and ownership assertions. All four
T820 integration tests pass. No production code or compatibility overload was
added. This repairs the test caller, not a new runtime behavior defect.

## T1732: current WebSocket and shared digest adapters

Update tokio-tungstenite/tungstenite from 0.21 to 0.30 and both direct SHA-2
consumers from 0.10 to 0.11.1 through a workspace declaration. SHA-1 0.11 and SHA-2
0.11 now share digest 0.11; no second digest/crypto-common/block-buffer family is
introduced. SHA-1 and SHA-2 remain distinct required algorithms. This removes two
more lockfile package versions: all-target total 542 -> 540, duplicate names stay
36. Upstream API references: <https://docs.rs/tokio-tungstenite/0.30.0/> and
<https://docs.rs/sha2/0.11.1/>.

Current WebSocket messages take owned Utf8Bytes/Bytes; callers transfer their
existing String/Vec payloads with Into. No old-version overload or shim remains.
Message/frame limits stay 64 KiB; read capacity stays explicit at 4 KiB pending
T1738's separate workload comparison. Sixteen admitted input handlers and the
existing bounded read-ahead queue retain their original cutoff and drop paths.
No extra capture, borrowed frame escape or detached cleanup owner is introduced.

Digest text now uses a library-independent `[u8; 32] -> String` helper. It owns
32 input bytes and produces 64 lowercase ASCII bytes in one allocation, retaining
no references or native resources. A known vector preserves order/leading zeroes;
a bounded property test checks all 256 byte values against Rust's independent
hex formatter. Device lock keys and cache fingerprints preserve their bytes.

Verification: Linux workspace all-target check passes; 166 input/route-lock/hash
library tests pass; 61 cache/credential binary tests pass (one pre-existing ignored
performance test remains ignored); three portable-session integration tests and
two actual private-display GUI startup/relaunch tests pass. Nine maintained
production functions have individually measured coverage between 91.4% and 100%;
exact counts and source hashes are in `artifacts/2026-11-22-dependencies/t1732-metrics.json`.
Local Rust syntax complexity remains <=9 throughout changed Rust files. Existing
Windows fixture constructors received mechanical current-API conversions only;
Windows native validation and feature work remain paused, with no support claim.
Release-bundle rebuild and notice attribution remain T1734 integration work.
