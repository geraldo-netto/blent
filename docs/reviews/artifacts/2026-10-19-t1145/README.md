# T1145 — owned output worker

`camera::output_worker::start` admits two original observations, then starts one
owned blocking worker and one async cancellation/join controller. Only that worker
clones the original lease, configures the same file and writes full frames through
the T1143/T1144 adapters. Returning from `start` admits work; it does not certify
that configuration succeeded. Native failure propagates through the controller.
The existing camera pipeline is replaced separately under T1137.

Controller destruction, including rejected admission or runtime teardown, closes
its stop authority. Cancellation before setup has no effect; cancellation during
setup retains the original file and task capacity until native return. Completed
setup rechecks cancellation before writes. Final black-frame cleanup on a running
output is also native work: a stuck write stays owned. The Report independently
observes both original completions. Neither an abort request nor a timeout releases
an unknown obligation. Eventual cleanup assumes native return and observer progress;
there is no unconditional cleanup claim for a permanently stuck kernel call.

The blocking worker owns its File/Arc lease, bounded black image and watch receiver.
A frame Arc lives through its synchronous borrowed write; no native pointer, callback
or extra output process escapes. The validated image bound is3110400 bytes. The
watch retains its latest frame and the active borrow; decoder-pool bounds remain
unchanged. Shared black/freshness policy has one implementation in `outputs.rs`.
No descriptor is reopened by pathname, and release/reuse requires actual native
completion and join. Kernel flock probes cover every tested release boundary.

Fourteen permanent worker tests cover start/stop, live/black frames, frame EOF,
five invalid frame lengths, six invalid profile boundaries, real ENOSPC/non-video
failure, rejected original observations, exhausted native/controller admission,
cancellation before/during setup and runtime teardown during blocked native work.
The setup gates are actual original kernel file locks. Representative native
interleavings are controlled explicitly; this is not exhaustive kernel verification.

A new fixture initially used a filesystem release marker that TempDir could erase.
The deterministic normal-suite regression failed before the fix, restored release
under an independent recovery sponsor, joined native work, then asserted. The exact
same regression function passes after the gate becomes an owned kernel lock whose
open identity survives path deletion. Its source/hash and red/green evidence remain.

Offline attested LLVM collection passes34 output tests. Serial replay of the same
ELF passes17 camera tests,2 private native lifecycle tests and3 original retirement
tests. All12 worker/shared-output production functions pass individually>=80%; the
five new functions each reach100%. This includes broader legacy-output counters,
not a full-host-suite or physical-camera claim. The first direct replay omitted
Cargo's established serial setting and reproduced documented T131 ETXTBSY. Its log
is retained; those failed-run counters are archived outside the final profile tree.
The accepted replay explicitly uses `--test-threads=1`.

Pinned source audit:63 authored Rust functions, maximum4. This is not SonarQube
measurement (T1019). Scoped host Clippy with `--no-deps` and rustfmt pass; excluded
EVDI dependency lint T1131 remains. Native driver primitives were qualified under
T1143/T1144; joined real-driver pipeline acceptance remains T1137. The source snapshot
also contains pending T1137 reproduction sources. Raw manifests/profiles/logs:
`/backups/disk2/blent-work/2026-10-19-open-items/t1145`. No transfer, install,
live-device/service change or push occurred.

```sh
cargo test --offline --locked -p blent --lib camera::output
cargo clippy --offline --locked -p blent --lib --features accessory-transport --no-deps -- -D warnings
```
