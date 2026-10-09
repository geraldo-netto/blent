# Owned C performance checks: T768, T769, T773, T778

This document records the initial research snapshot at `a3c053e`; its prototype
and open-item descriptions are historical unless an implementation follow-up is
explicitly identified below.

The measurements support improving conversion scheduling and evaluating portable
SIMD dispatch. They do **not** support replacing all spans with tiles or removing
GPU metadata checks. Production sources and release compiler flags are unchanged.

Evidence: [raw trials](artifacts/2026-10-09-c-performance/raw.json),
[summary](artifacts/2026-10-09-c-performance/summary.json),
[host, flags and source hashes](artifacts/2026-10-09-c-performance/metadata.json).
The source revision is `46ab552`; hashes also identify the then-uncommitted
benchmark sources. Results below are medians of five trial medians, in microseconds.
Each conversion trial uses 16 warmups and 96 samples. Comparisons reverse execution
order on alternating trials. Workloads run sequentially on one Ryzen 9 7945HX
host, without CPU pinning, frequency locking or an idle-host guarantee. Trial
ranges and p95 values are retained; small differences are not universal gains.

The replay uses 1920×1080 BGRA with 64 bytes of row padding, at scales 1 and 2.
It models empty damage, two distant 16×64 output rectangles, a full frame, a
full-width upper half plus a narrow lower half, one 16-pixel vertical strip and
many separated 2-pixel strips. It does not replay triple-buffer history rotation,
capture/encoder contention or a physical tablet. An independent scalar oracle
checks the complete output for every benchmark process, including unchanged
pixels. [36 ASan/UBSan cases](artifacts/2026-10-09-c-performance/sanitizers.log)
cover all workloads/scales/prototypes. The GPU metadata probe also
[passes ASan/UBSan](artifacts/2026-10-09-c-performance/gpu-sanitizers.log).

## T768: sparse damage amplification is real; blanket tiles regress

`pixel_damage_merge` retains one bounding interval per chroma row. Two distant
16-pixel rectangles therefore make the current code convert the entire width
between them. At scale 1 this fixture needs 2,048 source pixels but converts
122,880: **60× amplification**. This is conservative extra work, not lost damage.

The experimental alternative uses one 64-bit mask of 32-pixel tiles per chroma
row, combines adjacent tiles into runs, and reuses the production conversion
kernel for each run. It is deliberately confined to the tested widths; it is
not a complete replacement for the frame owner's histories or shared slots.

| Scale 1, one worker | Current span | Tile prototype | Result |
|---|---:|---:|---|
| Two distant rectangles | 37.633 | 5.421 | 6.94× faster; 4,096 pixels converted |
| Full frame | 627.963 | 634.917 | No demonstrated improvement |
| Narrow vertical strip | 24.688 | 42.081 | 70% slower; tile rounding doubles pixels |
| Many separated strips | 34.787 | 73.051 | 2.10× slower despite fewer pixels |

At scale 2, distant rectangles improve from 161.613 to 10.741 µs; the narrow
strip regresses from 24.117 to 46.580 µs. Short repeated kernel calls and loss of
long vector loops matter as well as pixel count. Metadata marking is sub-microsecond
in these fixtures and does not explain the large conversion differences.

For 540 active chroma rows, the current spans plus row mask use 4,388 bytes per
history; the prototype tile mask uses 4,320 bytes. At width 4096 the analogous
tile mask would need two words per row: 32,768 bytes for 2048 rows versus 16,640
for spans plus a row mask. Those are representation sizes, not measured process
RSS or a demonstrated production allocation saving.

Next: compare a small bounded set of exact spans with an adaptive fallback to
one interval. Account for marking, merging and all per-buffer histories. Before
production changes, retain regressions for distant/overlapping rectangles,
fragmentation fallback, scales 1–4, clipping/extreme coordinates, history
rotation and leased-slot immutability; require the same full-frame pixel oracle.
T768 remains open for implementation/evaluation of that tradeoff.

Implementation follow-up: T768 now keeps one enclosing interval and its largest
retained unchanged gap per row. It converts two exact spans only when the gap
is at least 128 output pixels and at least half the interval; otherwise it uses
one contiguous span. Histories remain bounded, producer-owned and attached to
their buffers/slots. A per-job kernel choice avoids gap checks on ordinary rows.
[Five implementation trials](artifacts/2026-10-09-t768/summary.json) improve distant
updates from 38.816 to 3.136 µs at scale 1 and 162.226 to 5.701 µs at scale 2
(one worker). Dense/narrow medians stay close to baseline; fragmentation adds
some metadata work. This is not an improvement claim for every workload.
Metadata grows from 8 to 16 bytes per chroma row: 4,320 extra bytes per 1080p
history, with no per-damage allocation. Permanent tests cover gap preservation,
bounded random unions, fragmented fallback, scales 1–4, threaded conversion,
padding, FIFO history rotation and retained shared slots. The initial large-gap
regression fails before the change and passes afterward.
The combined capture gate passes for all 184 maintained functions; T768 is resolved.

## T769: unequal row widths create unequal worker loads

`job_count` scans dirty rows and pixel widths separately, then `prepare_jobs`
counts rows again and partitions by row count. The benchmark alternative
combines counting and pixel accounting, then partitions contiguous rows by
their pixel weight. It preserves the job threshold, capacity limit, existing
worker synchronization and conversion kernel; only the benchmark uses it.

| Uneven damage, four workers | Current | Weighted prototype |
|---|---:|---:|
| Scale 1 wall conversion | 215.396 | 131.494 |
| Scale 1 process CPU per iteration | 412.323 | 433.815 |
| Scale 2 wall conversion | 334.155 | 180.989 |
| Scale 2 process CPU per iteration | 693.731 | 694.611 |

At scale 1 the current jobs own `[518400, 518400, 8640, 8640]` source pixels;
weighted jobs own `[264960, 264960, 261120, 263040]`. Wall latency improves 39%,
with 5% more process CPU. Scale 2 latency improves 46%, with nearly unchanged
CPU. Equal full-width rows already distribute well; no general speedup is claimed.
Eight-worker and empty/sparse cases are included in the raw results.

Scheduling alone falls from 0.885 to 0.712 µs in the scale 1 uneven case. Thus
the meaningful gain is load distribution, not the small scan reduction. A row
is still indivisible: weighting cannot balance a workload dominated by one row.
The benchmark's row width is a cost estimate, not measured per-row execution time.

Next: implement fused accounting/weighted partitioning within the existing pool,
after permanent regressions for exactly-once row coverage, empty masks, one row,
sparse gaps, unequal widths, all scales, NULL-mask semantics, capacity limits and
worker-creation failure. Preserve user-configured worker capacity and benchmark
both wall latency and CPU cost. T769 remains open until those requirements pass.

Implementation follow-up: T769 now uses fused accounting and weighted contiguous
partitions, preserving NULL-mask row dispatch and user capacity. The permanent
T769 regression fails on the original code and passes after the change; existing
conversion regressions and the 176/176 per-function capture gate pass.
[Paired implementation measurements](artifacts/2026-10-09-t769/summary.json)
cover 300 independent-oracle processes: skew latency falls from 222.122 to
133.446 µs at scale 1 and from 338.523 to 182.724 µs at scale 2 (four workers).
Scale 1 process CPU rises about 7%; scale 2 CPU is nearly unchanged. Dense work
has no demonstrated general gain. The T769 TODO row is resolved. The original
research runner now restores its pinned `a3c053e` sources from local Git so
private helper changes cannot silently alter the historical comparison.

## T773: ten requests per frame, but checks enforce real contracts

The probe executes the actual `check_geometry` twice on a private Xvfb output,
without opening DRI3, VAAPI or any physical display/render node. It tests both
name matching and a synthetic EDID property. It includes actual RandR transform
queries; native GPU adapter mocks are not used for these timings.

| Identity mode | X requests for both checks | Wall time |
|---|---:|---:|
| Explicit output name | 8 | 86.738 µs |
| EDID, as in supervised use | 10 | 105.504 µs |

Each check queries resources, output information, transform and CRTC geometry;
EDID adds a property query. Xlib already caches the interned atom after setup.
At 60 FPS the measured EDID pair is about 0.63% of a frame interval. This is
isolated client wall time, **not** process CPU or physical-driver latency. An
earlier exploratory run was slower, further reason not to generalize Xvfb timing.

`gpu_events_open` currently subscribes only to damage/cursor events in damage
mode and returns immediately in periodic mode. `events` consumes at most 256
events and does not handle RandR invalidation. Consequently, simply retaining
metadata until an existing event arrives would miss identity, CRTC, geometry or
transform changes. The post-fence check specifically rejects a transition while
the copy was queued; ordinary damage notification is not an equivalent contract.

Recommendation: keep the checks while evaluating any cache. A candidate needs
explicit complete invalidation for both cadence modes, queue-overflow handling,
and an ordering proof around copy completion. Required regressions include EDID
replacement with unchanged dimensions, output/CRTC replacement, rotation,
transform/position changes during copy, stale/queued events and reconnects.
No safe cache implementation or physical-driver speedup was established. T773
remains open as focused design/validation work, below measured conversion wins.

## T778: existing SIMD pays off; wider SIMD has workload limits

All 17 production C translation units were compiled separately with the release
optimization level. [Compiler diagnostics](artifacts/2026-10-09-c-performance/production-vector-summary.json)
identify automatic vector loops in BGRA/NV12 conversion, RGBA/BGRA swizzling,
cursor image copying, span initialization and EDID checksums. Therefore visible
C loops do not imply scalar machine code. The complete diagnostics are retained
as `production-vector.txt.gz`; benchmark diagnostics are retained per flag set.

| Full frame, one worker | Scale 1 | Scale 2 |
|---|---:|---:|
| `-O3 -fno-tree-vectorize` | 2416.569 | 1308.548 |
| Portable current `-O3` | 628.033 | 1279.262 |
| `-O3 -mavx2` experiment | 467.343 | 1244.344 |
| Explicit AVX-512 F/BW/VL experiment | 462.434 | 1195.971 |
| Host-specific `-O3 -march=native` | 322.844 | 1259.934 |

The existing unscaled conversion is **3.85× faster** with automatic vectorization
than with GCC vectorization disabled. AVX2 reduces its time by another **26%**;
the selected AVX-512 flags are only slightly better than AVX2 here. Native tuning
reduces time by 49%, but enables additional architecture features and tuning:
that extra improvement cannot be attributed solely to wider vectors. Generic
diagnostics show 16-byte/8-byte loop vectors; native diagnostics include 64-byte
and 32-byte vectors. These are conversion measurements, not whole-app FPS claims.

Scale 2 improves only about 3% with AVX2 and 7% with the explicit AVX-512 flags.
Narrow strips gain little or regress. The specialized downscaling loop and short
spans need separate treatment; blindly enabling wider vectors is not enough.

Other loop classes have different constraints: cursor blending has alpha-dependent
control flow; GPU cursor upload calls `XPutPixel` and is now cached by cursor
serial; polling, descriptor cleanup and worker/lease loops perform ordered native
operations rather than independent arithmetic. Row copies already use `memcpy`.
No cursor-blending or other unmeasured-loop speedup is claimed. Developer JNI
hash loops have a rolling dependency and are not part of the production hot path.

Next: evaluate **runtime-dispatched** AVX2 (and separately justified newer ISA)
kernels behind the existing conversion contract, keeping a generic fallback.
Do not put `-march=native` into portable release flags. Require permanent scalar
oracle tests for each forced dispatch path, odd/cropped/padded dimensions, tiny
spans, all scales, alignment, guards and invalid admission inputs; then rerun
per-function coverage and paired sparse/dense latency/CPU measurements. T778
remains open for that implementation and validation.

## Reproduction and scope

With the already-installed/cached development SDK selected through
`PKG_CONFIG_PATH` and `LIBRARY_PATH`, run:

```sh
GPU_CFLAGS=-I/usr/include/xcb python3 scripts/benchmarks/c-review-performance.py /tmp/blent-c-review-new --trials 5
```

The output directory must be new. The SIMD comparison deliberately requires a
Linux x86 host with AVX2 and AVX-512 F/BW/VL exposed by the OS; it checks this before
running ISA-specific binaries. `GPU_CFLAGS` is optional with ordinary installed
headers. The runner records exact compiler commands and source hashes and starts
private Xvfb instances. It never installs/downloads dependencies. Production
code, stock EVDI, live display state and Android installation were unchanged.
No multi-tablet or large-machine campaign was reopened.

One additional review-only finding, T777, records unchecked `atoi` arguments and
NDEBUG assertion-side-effect hazards in the older `damage_regions.c` benchmark.
The new replay uses the checked argument/build contract. Required failing
regressions and the older benchmark fix remain separately tracked.

## Nested loops and data structures: follow-up

Scale conversion visits each required source sample: O(output pixels × scale²),
which is O(source pixels) for a fixed source frame. The nested loops are not an
extra quadratic scan over the image. Replacing them with summed-area tables would
add a full-frame construction pass, storage and memory traffic even for tiny
damage; no measured benefit justifies that change. Scale-specialized kernels
already expose constant box sizes to the compiler.

Dirty history uses a row bitmap and T768's constant-size two-span representation:
O(1) merge per affected row, O(rows) scheduling, no per-damage allocation. Worker
partitioning scans contiguous rows once, rather than searching each job's prefix.
Raw shared slots have a fixed capacity of eight; their short linear scans avoid
an index whose maintenance would add complexity. Frame ownership changes use
pointer swaps. GPU event batches are capped at 256 and damage fetches use one
conservative extent. Cursor upload is already cached by serial. No evidence here
supports replacing these bounded structures with trees or hash tables.

T779 identifies the avoidable nested pass: each overlapping RGBA damage rectangle
previously repeated the pixel swizzle. A clipped-area budget now switches to one
full-frame swizzle when total requested work reaches the framebuffer area. This
bounds pixel work by O(frame pixels), plus at most 63 rectangle checks, without
allocations. The retained native image makes conservative copies valid; cursor
restoration precedes normalization. T779 regressions count actual copied pixels
and compare full channel order and padding under 256 bounded randomized rectangle
sets, including extreme and reversed endpoints.

T773 was rechecked against `host/gpu/events.c` and `capture.c`: periodic mode
still skips event initialization, and damage mode still lacks RandR invalidation.
The measured request cost is real, but a cache satisfying the required copy-order
and identity contracts has not been established. The owner's conditional approval
(less time without correctness drawbacks) is therefore not met. Existing checks
remain; T773 retains the exact research and acceptance prerequisites in TODO.md.

T779's [five paired trials](artifacts/2026-10-09-t779/README.md) at 1920×1080
reduce 63 full-overlap rectangles from 18,019.955 to 279.655 µs median wall time
(about 64×). Sparse 64×64 changes take 0.751 versus 0.771 µs; full frames take
280.136 versus 282.089 µs. The explicit tradeoff is fragmented bookkeeping:
63 narrow fragments take 10.370 versus 11.963 µs (+1.593 µs). There are no new
allocations. The full capture gate passes 185/185 individual functions at ≥80%;
ASan/UBSan and the red/green regression pass. T779 is resolved.

## T778 implementation follow-up: runtime AVX2

Runtime CPU/OS admission now selects AVX2 for unscaled work averaging at least
256 pixels per dirty chroma row. Portable and scaled kernels remain available;
shared arithmetic is inlined into ISA-specific entry points. There is no global
`-march=native`, allocation or duplicated pixel algorithm. The helper maps
`BLENT_CONVERSION_SIMD=auto|portable|avx2` into the independent conversion policy;
invalid/unsupported requests fail before capture resources open. The default is
auto; portable lets users avoid CPU-specific wider-vector tradeoffs.

[Final paired evidence](artifacts/2026-10-09-t778/README.md), five trials and 240
processes, gives these median microseconds at 1920×1080:

| Workload | Before wall | After wall | Before CPU | After CPU |
|---|---:|---:|---:|---:|
| Full frame, scale 1, one worker | 644.865 | 474.374 | 648.181 | 487.665 |
| Full frame, scale 1, four workers | 226.371 | 136.516 | 825.343 | 513.524 |
| Skewed damage, scale 1, one worker | 342.186 | 259.195 | 347.182 | 260.235 |
| Distant damage, scale 1, one worker | 3.216 | 3.246 | 3.418 | 3.410 |
| Full frame, scale 2, one worker | 1220.174 | 1230.695 | 1232.361 | 1256.726 |

The unscaled full-frame wall reduction is about 26% with one worker and 40%
with four. Scale 2 has no demonstrated improvement; it retains the same kernel
and shows small run-to-run differences. Narrow strips remain portable. This is a
conversion measurement on this CPU, not an app FPS or universal hardware claim.
Forced-path and disabled-AVX2 regressions, GCC/Clang ASan/UBSan, assembly checks
and the 192/192 individual production-function coverage gate pass. T778 is resolved.

T777's older damage replay now uses the shared checked numeric parser and rejects
NDEBUG builds. Permanent malformed/range/build-contract regressions and a valid
scale-4 replay pass in the normal benchmark test target; [red/green evidence](artifacts/2026-10-09-t777/README.md)
is retained. T777 is resolved; the earlier research-only description is historical.

## T782 implementation and larger-block review

AVX2 now reduces scale-2/4 boxes into two 128-pixel rows of worker-local scratch
(1 KB), then applies the existing color kernel. Each RGB output average truncates
before the four-pixel chroma average, preserving the independent oracle exactly.
Auto uses these reducers even for narrow spans; scale 3 remains portable.

[Five final paired trials](artifacts/2026-10-09-t782/README.md), median microseconds:

| Dense frame | Before wall | After wall | Before CPU | After CPU |
|---|---:|---:|---:|---:|
| Scale 2, one worker | 1222.759 | 257.679 | 1231.325 | 263.376 |
| Scale 2, four workers | 327.259 | 78.133 | 1275.512 | 289.857 |
| Scale 4, one worker | 955.701 | 132.807 | 961.134 | 136.283 |
| Scale 4, four workers | 258.922 | 45.758 | 1001.578 | 159.553 |

Scale 1 remains near baseline (469.672 versus 472.838 us, one worker). Gains
are conversion costs, not tablet FPS claims. All 198 individual native functions
pass coverage; permanent oracle and GCC/Clang ASan/UBSan tests pass. T782 resolved.

The requested 1/2/4/8/16 KB tile sweep found roughly 2–5% better dense medians
in some larger-tile cases, with overlapping trial ranges and no consistent sparse
benefit. The tile is stack memory accessed by ordinary loads/stores: there are
**no per-tile syscalls** to amortize. The selected default remains 1 KB. Increasing
it changes loop overhead, stack use and cache behavior, not an I/O transfer size.

The separate C/Rust I/O review found:

- C `fifo_writer.c::write_fifo_bytes` already requests the entire remaining frame;
  Rust `raw_transfer.rs::FrameWriter` likewise calls `write_all(frame)`. Short
  writes reflect backpressure, not a fixed 1 KB application chunk.
- C shared memory sends small slot descriptors through `sendmsg`; pixels remain
  in leased shared slots. Batching descriptors across frames adds waiting.
- Rust `stream.rs::write_frame_bytes` already uses vectored header/payload writes.
  Configuration packets still use three writes; that limited opportunity is T784.
- Rust's production framed/IVF packet readers request complete payloads through
  `BufReader`; large reads can bypass an empty buffer. Its 512 KB streaming
  Annex B reader is test-only. A measured 8/32/64 KB reader comparison is T785,
  not a proven bottleneck or an instruction to wait until buffers fill.
- Larger FIFO capacity already has a user control and [retained measurements](../benchmarks/2026-09-17-pipe-capacity.md):
  it can reduce write attempts/CPU but increase queued-frame age. The old tests
  do not justify a blanket default increase or frame batching.

## T783 implementation: native channel order

Conversion now takes an independent BGRA/RGBA pixel-order contract and selects
specialized kernels once per batch. Cursor images remain BGRA; blending respects
the borrowed surface order and restores original bytes. Default native capture
registers one source framebuffer. `BLENT_CAPTURE_FORMAT=canonical` retains the
separate swizzle path for comparison/troubleshooting and its permanent regressions.

[Five paired trials](artifacts/2026-10-09-t783/README.md) include normalization,
cursor restore/draw, damage, conversion and publication with mocked EVDI readback.
At 1920×1080 RGBA with cursor and one worker, dense wall medians fall from
760.178 to 479.230 µs (scale 1), 533.557 to 260.303 µs (scale 2), and 405.390
to 133.289 µs (scale 4). One 8,294,400-byte allocation is removed. BGRA already
avoids the pass; its dense comparison ranges from −0.2% to +2.4% with one worker,
so no BGRA speedup is claimed. Specialized channel kernels increase code size.

The new single-buffer regression failed before the change and passes afterward.
Permanent scalar-oracle, format/resize/cursor/lease and invalid-order tests pass,
as do GCC/Clang ASan/UBSan and all 207 individual native coverage checks. T783
is resolved. These are owned userspace timings, not compositor or tablet FPS.
