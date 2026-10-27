# T819: optional native encoder isolation

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

Native libavcodec now runs in one supervised Blent child per session. The existing
raw memfd slot/final-reference protocol crosses exec without another raw-frame copy.
An independent bounded job/packet contract is separate from the Linux adapter.
Only compressed output crosses a pipe (vectored header/payload writes). Parent
queues retain their existing budget and generation invalidation. The historical
feature switch is still `inproc-encoder`; default FFmpeg CLI builds are unchanged.

Child death, stopped children, cancellation and restarts retain FIFO/admission
ownership until actual reaping. A parent-generated nonce prevents a replacement
from accepting a predecessor's raw handshake. Parent death kills the worker.
Final Linux runtime shutdown has a configurable 1,000 ms default, 1..60,000 ms
`BLENT_RUNTIME_STOP_MS`, so a started blocking call cannot hold final exit forever.
This is process fault containment, not recovery from GPU/kernel-wide stalls.

## Paired measurements

Release build, stock local SDK libx264, deterministic shared raw producer, 150
frames/run, five alternating direct/isolated pairs per resolution. Every pair has
identical concatenated encoded output SHA256. Timings cover raw publication to
parent compressed-packet receipt, not Android display or physical EVDI capture.
Medians across five runs:

| Resolution | Direct / isolated startup, ms | Packet p50, ms | Packet p95, ms | Parent+encoder CPU per run, ms | Parent RSS, KiB | Isolated worker RSS, KiB |
|---|---|---|---|---|---|---|
| 1280×720 | 9.247 / 23.792 | 1.619 / 1.511 | 1.841 / 1.725 | 217.911 / 235.499 | 60252 / 51620 | 53304 |
| 1920×1080 | 9.715 / 24.612 | 2.793 / 2.752 | 4.306 / 4.182 | 449.270 / 463.739 | 77800 / 59176 | 67904 |

CPU includes startup/cleanup, excludes the producer process. RSS is sampled resident
memory per process; adding it double-counts shared pages and is not PSS/GPU memory.
Accept for containment, not a general speedup: startup, memory and CPU increase.
Steady latency differences are small and workload-specific. Existing direct runner
is test-only for paired comparisons; production always uses the child.

## Permanent validation

`host/tests/runtime_exit.rs` failed before explicit runtime ownership and passes
afterward. Native tests retain malformed/oversized packet/job/descriptor bounds,
partial and interrupted writes, lost parent, missing executable, SIGSTOP/SIGKILL,
restart nonce, immutable headers and retired generation tests. Existing FIFO and
native shared-lease regressions remain. 53 scoped native tests pass (2 preexisting
ignored); default host library 330 and binary 416 pass. Broad default tooling has
two known packaging-prerequisite failures (T833; deliberately not pursued). Windows tests cross-check;
native Windows runtime remains T737. All 67 production functions in the affected
native adapter/contract/runtime/capture/raw modules meet 80% line coverage and CC≤9.

Evidence: [logs and per-function report](artifacts/2026-10-10-t819/). SDK tests and
builds used installed tools with network disabled. Reproduce measurements inside
that SDK by building the release production binary, then running the native binary
test `t819_worker_matches_inprocess_shared_output_with_paired_measurements` with
`BLENT_T819_MEASURE=1280x720` or `1920x1080` and `--nocapture`. No installation or
physical device test was performed.
