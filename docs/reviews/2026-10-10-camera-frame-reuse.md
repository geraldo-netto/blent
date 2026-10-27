# T816: bounded camera frame reuse

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

The decoder reuses three immutable frame slots by default, plus one fixed discard
frame. `Arc::get_mut` requires exclusive ownership: outstanding output/preview
readers and weak references prevent reuse. Full pools drain one frame without
publication, preserving byte framing and bounded memory without waiting on a
reader. A new connection owns a new pool. Partial/EOF/cancelled reads never publish.
There are no per-frame Vec or Arc allocations after initialization.

Permanent tests cover slow readers, full pools, resumed reuse, generation changes,
Stop, partial input, cancellation, invalid lengths/capacities and a bounded randomized
lease schedule. Existing camera codec/transform/native-lifecycle tests pass (25
before the extra schedule property; four focused T816 tests pass afterward).
All seven production functions in frame_pool/decoder meet 80% line coverage and
CC≤9. This is an optimization with new bounded-pressure handling, not an existing
behavioral bug fix; tests were added before switching the decoder to the pool.

Five alternating release pairs, 300 frames each, memory-backed exact reads and
real latest-value publication. The retained benchmark has a counting allocator which forwards zeroed allocation
to `System::alloc_zeroed` unchanged;
steady pooled allocation count/bytes must remain zero. At 1080p, p50 falls from
65.858 to 44.647 µs, p95 from 70.517 to 48.935 µs; CPU per 300 frames from
21.307 to 13.494 ms. Baseline allocates 600 times / 933,132,000 requested bytes;
pooled steady state allocates zero. These are allocation traffic, not retained RSS.

Pooling deliberately retains four frame buffers per connection: 5.53 MB at 720p,
12.44 MB at 1080p plus metadata. Initialization does extra one-time allocation;
1080p median initialization was 2.962 ms. This trades bounded retained memory
for lower allocation traffic and stage CPU. No whole-camera speedup is claimed.
Full raw pixels still cross the FFmpeg pipes; this is reuse, not zero-copy decoding.

[Raw runs, medians, tests and per-function coverage](artifacts/2026-10-10-t816/).
Reproduce with `BLENT_T816_MEASURE=1 cargo test --offline --locked -p blent
--release --test camera_pool -- --nocapture`. No physical camera/output used.

The all-feature/all-target offline SDK check also compiles the retained camera
feedback example. It initially failed after adding the pool dependency; the example
now includes that production sibling, and the same check passes. See example-red
and all-targets-green logs. No physical probe was run.
