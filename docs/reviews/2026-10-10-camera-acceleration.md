# T817: camera hardware/fusion evaluation

Decision: retain stock software camera decoding and separate output owners.
VAAPI decode/readback and the tested fused split are not accepted for production.
This is a measured rejection, not a claim that all hardware acceleration is slower.
No EVDI/dependency changes, driver installation, downloads or real V4L2 writes.

## Hardware replay

Both locally accessible AMD render nodes initialized stock VAAPI successfully.
48 runs: 60 paced H.264 frames at 30 fps, 720p/1080p, unrotated/90° rotation with
scale/pad, three alternating-order runs per variant. Compare software, VAAPI decode
plus CPU readback/filters on both nodes, and unrotated VAAPI scale plus readback.
All runs deliver exactly 60 YUV420 frames with identical SHA256 within each geometry/
transform case. CPU covers the FFmpeg child; RSS is sampled process memory, not GPU
allocations. Delivery age starts before compressed packet write and ends at complete
raw-frame receipt; it includes pipe/probe buffering, excludes Android/network and
V4L2 consumption. P95 includes startup, deliberately not just warmed steady state.

Median of three runs, software / renderD128 / renderD129:

| Case | Packet-to-frame p50, ms | P95, ms | Startup, ms | FFmpeg CPU per 60 frames, ms | Peak RSS, KiB |
|---|---|---|---|---|---|
| 720p | 36.46 / 38.34 / 38.14 | 37.86 / 108.71 / 103.94 | 125.24 / 209.66 / 202.25 | 306.45 / 364.52 / 363.60 | 70520 / 127628 / 128236 |
| 1080p | 39.36 / 41.15 / 41.28 | 40.88 / 120.62 / 119.36 | 138.56 / 222.22 / 218.44 | 481.72 / 482.12 / 482.93 | 83996 / 139124 / 138328 |
| 1080p rotated | 43.02 / 44.47 / 44.48 | 47.40 / 100.29 / 124.20 | 146.58 / 203.74 / 224.15 | 699.74 / 608.23 / 647.73 | 84724 / 140008 / 140004 |

Unrotated hardware scaling did not improve these results: 1080p p50 41.31/41.29 ms,
p95 121.00/118.02 ms, CPU 492.13/492.50 ms. No scaling was needed in this case;
rotation/aspect-correct scaling was separately measured through the CPU filters.
The 720p rotated renderD129 case had a lower median CPU (230.64 vs 417.60 ms), but
one of three runs regressed to 473.43 ms CPU and 110.15 ms p95. All hardware cases
retained roughly 55–59 MiB additional process RSS. Some CPU savings are real for
these runs, but neither broad latency improvement nor a cost-free default is
established. A future hardware-specific CPU/battery tradeoff would need its own
explicit capacity/fallback policy and acceptance; none is silently enabled here.

## Fused worker rejection

The reproducible stock FFmpeg `split` prototype sends the same filtered frame to
preview and an unread output pipe. At 160×120, separate bounded fanout completes
300 preview frames in 80.39 ms; fused output delivers 12 then stalls through the
800 ms observation. Both first bytes arrive around 51–59 ms. The parent retains
one current sink value in the independent replay; it models latest-value fanout,
not an actual V4L2 device. The fused output is a real unread pipe. This rejects the
shared-backpressure design before further throughput tuning. FFmpeg's queues
initially hide the coupling; eight-frame probes were too short to expose it.

The permanent normal-tooling replay uses a two-second bound for slower CI machines,
requires all 300 independent preview frames, and requires stalled fused progress
to remain incomplete. Existing camera failure, format, immutable-reader, Stop and
process-retirement tests remain unchanged. No new runtime path requires fallback:
the already tested isolated software pipeline remains the only production path.

[Full commands, hashes, runtime versions and evidence](artifacts/2026-10-10-t817/).
Reproduce with `python3 scripts/benchmarks/camera-decode.py /tmp/new-camera-run`
and `python3 scripts/benchmarks/camera-fusion.py`. Inputs are generated locally.
