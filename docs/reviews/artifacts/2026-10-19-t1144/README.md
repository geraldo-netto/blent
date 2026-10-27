# T1144 — whole-frame output capability

The output writer sends one complete frame per native write and becomes unusable after uncertain native failure. Byte-boundary, ownership and isolated-driver tests passed; full camera integration was still separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`camera::output_writer::Writer` consumes one T1143 `ConfiguredOutput`. It validates
exact YUV420 frame length, attempts one synchronous native write and rejects all
later writes after any native error, short/zero write or unwinding native port.
Invalid length has no native effect and leaves a healthy writer usable. There
is no pathname lookup, extra output process, background work or frame allocation.
The camera pipeline still uses its old path-based producer until T1137 integration;
this commit qualifies an adapter, not the complete camera lifecycle.

The original FFmpeg raw-output draft failed the permanent packet/count/length/byte
check: two115200-byte frames became eight writes (32768/32768/32768/16896 each).
Direct/flush/packet-size, fd protocol and nested muxer probes did not correct it.
The historical file named `t1144-packet-green.log` is **also a failure**, retained
as `direct-flags-still-red.log.gz`. No successful FFmpeg qualification is claimed.
Original draft source and red logs are retained. The test's transport scaffolding
changed to the synchronous writer; its `assert_frames` byte/count/length oracle
is unchanged, with hashes in `packet-oracle.json.gz`. The normal suite also retains
a deliberately fragmented native control that the same oracle must reject.

Permanent tests cover native SOCK_SEQPACKET write boundaries and EOF; every short
length and the first overlong length in0..115201 (115201 invalid cases); six
short/zero/out-of-range return counts; five OS errors including EINTR; panic;
real EBADF; and kernel flock ownership through success, rejection and failure.
The writer owns its original File until drop after the native call settles.
A borrowed frame cannot escape that synchronous call. Healthy/poisoned transitions
prevent native reentry after uncertainty. Rust ownership prevents accessing a
released File; lease probes independently verify release. The containing worker
must be admitted before configuration/access, retain ownership through a blocked
call, and join before reuse. Caller cancellation or a timeout alone is insufficient;
that integration remains T1137. Eventual cleanup assumes native return/unwind or
actual OS process teardown; no unconditional cleanup of a stuck syscall is claimed.

Offline native LLVM collection passes20 host tests. The identical attested ELF
also runs in two isolated guests: real v4l2loopback0.15.3 captures unchanged
115200-byte frames at30/5/7/30FPS, and the retained T1143 independent C query verifies
format/size/rate. The frame fixture first publishes a frame before initializing
capture reads; an earlier fixture attempted reads before the driver was ready.
Both final guests retire the module and exit. These are Linux x86_64 synthetic
results, not physical camera/tablet, other kernel, musl or other-platform acceptance.

Combined source/ELF-attested counters pass20/20 production functions individually
at>=80%; the three new writer functions each reach100%. Pinned parser source audit:
20 Rust functions, maximum5; this is not SonarQube evidence (T1019). Scoped host
Clippy with `--no-deps` and rustfmt pass. Dependency-wide Clippy still encounters
excluded EVDI lint T1131; it was not suppressed or changed. Raw manifests,
profiles, runner metadata and logs remain under
`/backups/disk2/blent-work/2026-10-19-open-items/t1144`. The snapshot includes pending
T1137 reproduction sources; the coverage claim is restricted to these20 functions.
No downloads, uploads, live media/service changes or pushes occurred.

Reproduce the normal adapter suite offline:

```sh
cargo test --offline --locked -p blent --lib camera::output
cargo clippy --offline --locked -p blent --lib --features accessory-transport --no-deps -- -D warnings
```

Use T1143's documented offline guest runner with suite
`camera::output_device::tests::virtual_driver::t1144_virtual_frame_transfer_fixture`
for the frame test and its default suite for independent metadata. Wait for the
foreground VM's actual exit before running `check`; a QEMU exit alone is not a pass.

</details>
