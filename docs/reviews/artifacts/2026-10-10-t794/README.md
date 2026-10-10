# T794 mapped VAAPI input research — 2026-10-10

Research probes only; no production path changed. Stock EVDI and dependencies
untouched. All work used the installed SDK with network namespace disabled.

## Disposition: reject this candidate at the early performance gate

T794 is resolved by evidence-backed rejection, as permitted by its acceptance
contract and the owner’s instruction to reject measured regressions. The
[implementation plan](../../2026-10-10-t794-implementation-plan.md) remains
a historical design for reconsideration, not shipped functionality.

The candidate saves wall time but increases CPU consumption. Keep existing upload
and all existing fallbacks. No production integration is justified by these data.
The pinned discrete-node measurements are stable enough to reject independently
of the much noisier integrated-node runs:

| renderD128, pinned, 5 alternating pairs ×200 frames | Upload | Mapped | Change |
|---|---:|---:|---:|
| Median of run median total latency | 3.471 ms | 2.878 ms | −17.1% |
| Median of run p95 total latency | 3.635 ms | 3.406 ms | −6.3% |
| Median of run median producer + codec CPU/frame | 1.444 ms | 1.567 ms | +8.5% |

Every individual D128 paired CPU median increased: +7.33%, +7.90%, +8.58%,
+11.57%, +6.90%. This exceeds the proposed plan's 5% regression gate on all
five pairs despite favorable latency results. Mapped conversion alone increased
from 202 to 334 microseconds, while explicit upload was removed. These CPU values
are CPU-time per frame, not CPU percentage or throughput.

D129 paired CPU changes were +7.20%, +12.99%, +9.91%, +3.37%, +95.07%; the
median paired increase was +9.91%. Absolute run times varied strongly, so do not
present the ratio of pooled medians there as a stable effect estimate. The D129
same-process exploratory run independently recorded CPU regressions +7.1…13.2%
in all five pairs. Raw runs, including unfavorable/noisy results, are retained as `.gz` files.
Encoded files remain reproducible from the probes; their hashes and decoded
frame hashes are retained rather than every duplicate bitstream.
No claim is made about the cause of changing run states.

The probe fails qualification before production integration. Padded/sparse native
correctness passes; missing broad production qualification is not called a pass.
This disposition rejects this tested candidate, not every possible future mapped
implementation. Reconsider only with new stock-driver/hardware or implementation
evidence that removes the CPU regression while preserving correctness/isolation.

## Evidence and scope

`mapped.c` is a favorable same-process microbenchmark. Both paths use current
Blent conversion kernels, four conversion workers, identical deterministic BGRA
input, H264 VAAPI constrained baseline, QP18, CQP, async depth 1, no B frames,
60-frame GOP, BT709. Baseline writes an owned CPU NV12 frame then uploads it;
candidate writes directly through WRITE|OVERWRITE|DIRECT mapping and unmaps.
Twenty warmup frames precede 100 measured frames; each iteration has a 10 ms
sleep. Input painting is outside measured time. Packet output is included in CPU
time, outside the packet-return timestamp. The initial 5-pair results are in
`pairs`; runs alternate upload/mapped and mapped/upload. All 10 encoded outputs
per node have identical SHA256; independently decoded frame hashes match.

`isolated.c` retains producer/codec process separation, uses 3 finite surface
slots and imports each descriptor once through Unix SCM_RIGHTS. Producer and
codec open their own device contexts after fork. The CPU baseline borrows one
anonymous shared-RAM NV12 view directly for av_hwframe_transfer_data: no extra
CPU-frame allocation or memcpy. Candidate maps/converts/unmaps producer-owned
surfaces, then sends a slot request. Both paths use the real conversion kernel.
The consumer synchronizes the input surface while its frame is retained, drops
the frame, checks the imported-frame final-reference callback, and only then
acknowledges reuse. Parent and child each have a 30-second alarm. GPU kernel
hangs are outside what an alarm can guarantee.

`final-pairs` is the unpinned 100-frame run. `pinned-pairs` repeats five alternating
pairs at 200 measured frames with process/thread affinity restricted to logical
CPUs 8–11 (four distinct physical cores). The latter controls migration more
closely without changing governor or host settings. First exploratory isolated
results in `isolated-pairs` overlap a separate Gradle build and are not decision
evidence. Retain them explicitly labelled, not silently mixed with final runs.

All timing JSON inputs use nanoseconds. `summarize.py` writes microseconds with
`*_us` names. It reports per-run medians, rank-based p95/p99, individual paired
percentage changes and median of run medians. CPU sums CLOCK_PROCESS_CPUTIME_ID
in producer and child, so includes conversion workers. Total latency begins
before conversion and ends at completion acknowledgment. These are native
microbenchmarks, not production capture-publication-to-parent packet latency:
production supervisors, shared-ring state, damage tracking, EVDI readback and
Android decode/render are absent. Do not claim a production speedup.

`share.c` checks native sharing and preservation separately from timing. Three
surfaces rotate for 18 frames. First use fills the full visible image; each reuse
updates disjoint patches. Full mapped readback compares every visible Y/UV byte
against each slot's CPU history. Producer exports actual DRM PRIME 2 composed
layout plus object FDs; consumer imports it on the same render node. Import
surfaces stay alive through codec-reference retirement and device completion.
Run both nodes at 1280×800 and padded 1282×802. Upload and mapped-import streams
are bit-identical, and separate FFmpeg decoded frame hashes match all four cases.
The correctness-only upload helper copies expected pixels into its CPU frame;
that helper is **not** used for performance measurements.

`share-write-only-failure.log` retains an unsuccessful sparse experiment: mapping
WRITE|DIRECT without OVERWRITE did not preserve untouched bytes (first reuse,
sequence3, row0 was0 instead of16). Adding READ to sparse mapping preserves them
and passes. This is API feasibility evidence, not a claim of a stock-driver bug.
Sparse paths must explicitly establish preservation; removing OVERWRITE alone
was insufficient on the tested stock stack.

No production APIs, fallback policy, GPU vendor admission or installation changed.
No claim covers other GPUs. Padded/sparse checks establish correctness only, not
performance. Full pipeline, idle/1080p/held-slot performance, allocation/PSS/GPU
memory accounting, fault injection and host/Android acceptance remain unmeasured
because this candidate is being evaluated at an early research gate.

## Reproduction

From the repository root, with the retained files in `PROBE_DIR` and an empty
writable result directory, run the installed SDK without downloads:

```sh
flatpak run --unshare=network --device=dri --command=sh \
  --filesystem=/backups/disk2/projects/blent --filesystem=/tmp \
  org.freedesktop.Sdk//25.08 -c \
  'cd /backups/disk2/projects/blent; docs/reviews/artifacts/2026-10-10-t794/run-sdk.sh /tmp/blent-t794-reproduce'
```

Adjust paths to the retained artifact directory. `T794_CPUSET` defaults to 8–11;
choose four valid distinct physical cores when reproducing on other hardware.
`T794_FRAMES` defaults to 200. Render nodes are deliberately explicit: 128 and129
in this local research script, not production discovery. Do not run alongside
other benchmarks or builds. No host process/service/configuration is changed.

Compiler GCC15.2.0, stock FFmpeg7.1.3 (libavcodec61.19.101,
libavutil59.39.100), libva1.22.0, libdrm2.4.134; full versions in provenance.log.
Host kernel7.0.0-38-generic. AMD renderD128=PCI1002:73FF at0000:03:00.0;
renderD129=PCI1002:164E at0000:08:00.0. Both use amdgpu. Those identifiers support
reproduction only; vendor identity is not a capability gate.

## Validation

ASan+UBSan instrumented the project-owned probe/conversion/encoder helper code,
with stock libraries left uninstrumented. Both nodes passed imported/CPU paths at
padded1282×802 (18 sparse frames), and isolated timed-path smoke at1280×800
(20warmup+5measured) without sanitizer diagnostics. Leak detection disabled for
external driver caches; this does not claim a leak check. Commands used
`-O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer`,
`ASAN_OPTIONS=detect_leaks=0 UBSAN_OPTIONS=halt_on_error=1`; logs in sanitizers.

All new retained C functions have measured cyclomatic complexity≤8; benchmark
Python and shell helpers≤4. Benchmark coverage exemption applies; no production
behavior changed and no behavioral-regression fix is claimed. CPU metric includes user and system CPU charged to the measured processes,
including stock-library calls and conversion threads. It does not measure GPU
execution or CPU charged to other processes or independent kernel workers.
