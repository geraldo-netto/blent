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

## T1738: measured WebSocket read capacity

Keep 4 KiB for interactive input. 128 KiB improves large synthetic bursts but
regresses the representative small-frame workload. The benchmark is retained as
`cargo run --release -p blent --example websocket_buffers -- --buffer-kib 128`;
`--payload`, `--clients`, `--frames` and `--burst` select bounded workloads.

72 fresh-process runs: twelve alternating pairs for each of three workloads,
CPU affinity 2,4, two Tokio workers, AMD Ryzen 9 7945HX. Every received payload is
compared against the transmitted bytes. Each case owns its listener/socket pair;
all task outcomes are joined, including failures. No tablet service is accessed.

| Workload | 4 KiB median payload throughput | 128 KiB | Interactive p95, 4 / 128 KiB |
| --- | --- | --- | --- |
| 200-byte interactive, 8 clients × 10,000 frames | 39.59 MiB/s | 33.67 MiB/s | 50.32 / 62.10 µs |
| 200-byte bursts, 8 clients × 10,000 frames | 146.80 MiB/s | 109.68 MiB/s | — |
| 64-KiB bursts, 4 clients × 1,024 frames | 2625.47 MiB/s | 2892.11 MiB/s | — |

Interactive process VmHWM medians were 5260 / 5620 KiB. Reserved server read
capacity for eight connections is 32 / 1024 KiB; this is distinct from measured
whole-process residency. Final observations use Linux VmHWM because getrusage's
high-water mark can retain launcher memory across exec. An initial unpinned pilot
was exploratory and is not used for these numbers. Other platforms report no
memory measurement. Raw observations and source hash are retained in
`artifacts/2026-11-22-dependencies/t1738-results.json`.

The comparison concerns buffer sizes within tungstenite 0.30, not old-versus-new
library performance or physical USB/Wi-Fi performance. Camera/display pixels do
not travel through this input WebSocket. Nine benchmark functions were measured
by the local Rust syntax checker (syn 2.0.119); maximum cyclomatic complexity 7.
Benchmarks are exempt from production line-coverage gates under AGENTS.md.

## T1739: mp4ra workspace notice qualification

mp4ra-rust 0.6.0 omits standalone license files from its crate, but its published
`.cargo_vcs_info.json` identifies revision 3fc2d3a508b7867f3aa981ca902b24260fb0223f.
The upstream workspace root at that exact revision contains LICENSE-APACHE and
LICENSE-MIT. Their unchanged bytes, source URLs and hashes are pinned in the
supplemental catalog, together with the crate checksum and normalized manifest
identity. No implementation or cached source was changed.

The permanent offline T1739 regression uses the published manifest/VCS metadata.
It failed before the pin with `no crate notice files found`; the same test now
passes and verifies both notices in the archive and notice directory, their
revision, and unchanged fixture inputs. All 16 supplemental-notice tests pass,
including identity rejection, staging cleanup and partial-failure coverage.
Existing production collector functions are unchanged. Python 3.12 AST complexity
measurement covers all 21 functions in the changed test module, maximum 3; the
new test scores 2. The bounded test owns
its temporary archive/notice directories through unittest cleanup; no reference
escapes them. hex-slice remains separately unresolved in T1463.

## T1733: remaining duplicate constraints

The exact Linux normal/build graph for the host's installed features and GUI
contains 334 package versions, down from 345 at 90a56e29. Names with multiple
versions fall from 20 to 13. This includes build dependencies; it is not a count
of linked runtime libraries. `artifacts/2026-11-22-dependencies/release-graph.json`
retains both selected graphs and the exact delta. The full all-target lockfile
falls from 551 to 540 versions and from 43 to 36 duplicated names.

| Retained duplicate names | Consumers / removal constraint |
| --- | --- |
| smithay-client-toolkit, calloop, calloop-wayland-source, rustix, linux-raw-sys | Current winit 0.30.13 uses toolkit 0.19/calloop 0.13/rustix 0.38; current smithay-clipboard 0.7.3 uses toolkit 0.20/calloop 0.14/rustix 1. Current eframe 0.37 still uses winit 0.30. These require upstream API alignment; disabling Wayland/clipboard would remove capabilities. |
| getrandom | ring 0.17.14 requires 0.2, ahash 0.8.12 requires 0.3, tempfile 3.27 and rand 0.10 use 0.4. Different API generations cannot be unified by a Cargo override. |
| hashbrown | AccessKit consumer 0.28 uses 0.15; indexmap 2.14 uses 0.17. Current eframe 0.37 -> AccessKit Unix 0.23 -> AT-SPI common 0.20 -> consumer 0.39 can align this family. GUI migration plus the Rust 1.95 minimum (packaging currently 1.90) remains T1740. |
| thiserror, thiserror-impl | winit/toolkit 0.19 and existing AccessKit consume 1; newer toolkit and WebSocket stack use 2. Updating only Blent's direct declaration cannot remove the old consumers. |
| toml_datetime, toml_edit, winnow | TOML 0.8/current native-audio system-deps 6 retain the older parser family; proc-macro-crate uses the current parser family. T1731 preserves the explicit audio-work pause. |
| shlex | bindgen 0.69 under libspa-sys/pipewire-sys 0.8 consumes 1.3; current cc consumes 2.0. This build-only chain requires native-audio dependency resumption. |

Primary release references: <https://docs.rs/winit/0.30.13/>,
<https://docs.rs/smithay-clipboard/0.7.3/>,
<https://docs.rs/eframe/0.37.0/>,
<https://docs.rs/accesskit_atspi_common/0.20.0/>.
Exact cached normalized manifests and Cargo inverse trees verify these edges.
No dependency is removed solely from the notice inventory. The review is complete;
separate actionable framework migration and paused TOML work remain visible.

## T1741: independently delegated intermediate buffer comparison

Retain the 4 KiB default after the owner-requested intermediate-size investigation.
The subagent completed 588 measured fresh-process runs plus 30 excluded warmups:
4/8/16/32/64/128 KiB, a separate 4/6/8/12 KiB confirmation, then 60 final small-burst
pairs at 4/8 KiB. No observation was discarded. 8 KiB is the closest measured
compromise, but not an improvement across the representative workloads.

In the independent finer sweep, paired 8/4 medians show interactive throughput
-0.4%, p95 latency +0.8%, and 64 KiB bursts +5.4%. The final small-message burst
confirmation suggests -4.9% throughput (95% paired-bootstrap interval -8.3% to
+1.0%). That uncertainty cannot establish parity or exclude a material regression.
The input channel primarily carries small control messages, not video pixels.
No production capacity or 64 KiB protocol limit changed, and no hardware speedup
is claimed. 8 KiB would cost 4 KiB more reserved capacity per handler, separately
from observed process residency.

[Retained report](artifacts/2026-11-22-dependencies/t1741/report.md) contains every
stage, raw compressed records, summaries, exact commands, code hashes and limits.
The active workstation and possible initial Gradle overlap are disclosed;
subsequent Gradle verification used separate CPU affinity. All 12 runner/analysis
functions measure complexity <=6 under the local Python 3.12 AST checker.
No running Blent process or device was touched. This resolves the requested
comparison without changing the default on an unsupported performance claim.

## T1742: owner-selected 16 KiB default

The owner selected 16 KiB after reviewing T1741's tradeoffs. This supersedes the
previous 4 KiB default recommendation; it is a preference decision, not a newly
measured performance improvement. The input WebSocket adapter now requests 16 KiB
while retaining the 64 KiB message/frame limits and 16-handler admission ceiling.
Configured read capacity is therefore at most 256 KiB across admitted handlers,
192 KiB more than the previous setting; actual whole-process residency and other
transport/payload allocations are separate. Ownership, cutoff and socket drop
paths are unchanged. Running sessions were not restarted.

All 159 selected input tests pass, with no failures or ignored cases. The changed
connection handler has 53/53 executable lines covered and complexity 6 under
the local Rust syntax checker. Exact evidence is retained in
`artifacts/2026-11-22-dependencies/t1742.json`. This source change has not yet been
installed; deployment remains part of T1734.
