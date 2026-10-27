# T818: reusable audio staging and identity-rate bulk copy

Audio staging reused connection-owned buffers and bulk-copied the exact identity-rate case while preserving interpolation and atomic validation. Benchmarks measured small stage costs, not acoustic latency. Scoped counters were kept separate from an unrelated invalid full export.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Microphone input now reads into one fixed, connection-owned native packet buffer.
A portable `PcmView` validates generation, direction, length, sequence, timestamp and
optional clock before advancing state. Native little-endian PCM bytes remain in
place; the adapter no longer creates a combined packet, materializes an unused
PCM block, then allocates another native packet. The borrowed output cannot be
mutated before its consumer finishes. Partial reads still retire the connection
under the existing whole-packet deadline.

Speaker capture retains its bounded native buffer and reuses wire serialization
storage. Writer validation completes before changing the caller's buffer or sequence.
All bytes, including reserved flags, are overwritten on success. Existing allocating
APIs delegate to the same serializer, so byte layout and rejection policy have one
owner. Stereo/interleaving, little endian and v1/v2 clock layout are unchanged.

For exactly zero phase and zero drift, the worker queue copies at most two contiguous
PCM slices, including integer offsets retained from earlier drift. Fractional/drift
rendering retains the scalar interpolation path. Portable slice copies allow the
compiler/libc bulk-copy implementation; no architecture-specific SIMD dependency,
callback allocation, extra queue depth or new thread is introduced.

## Measurements

Release, same local machine, five alternating before/after render process pairs;
each process runs five 20,000-block samples per direction. Staging uses five
alternating in-process pairs, 10,000 packets per direction/variant. Counts come from
a test allocator forwarding all allocation kinds unchanged to System. Raw staging
reads use memory-backed AsyncRead, not sockets; these are stage costs.

| Stage | Before / after p50 | Before / after p95 | Steady allocations per 10,000 packets |
|---|---|---|---|
| Mono receive | 120 / 50 ns | 140 / 61 ns | 20,000 / 0 |
| Stereo receive | 161 / 60 ns | 200 / 90 ns | 20,000 / 0 |
| Stereo wire serialization | 80 / 30 ns | 90 / 40 ns | 10,000 / 0 |
| Mono identity render | 2.605 / 0.030 µs | 2.706 / 0.040 µs | already allocation-free |
| Stereo identity render | 3.216 / 0.050 µs | 3.346 / 0.051 µs | already allocation-free |

Stereo receive allocation traffic drops from 38.69 MB to zero per 10,000 packets;
serialization drops from 19.48 MB to zero after initial capacity. The identity
benchmark includes byte-exact sample assertions. Staging's reference retains the
old adapter's allocation/copy algorithm while sharing current validation, isolating
storage costs. Timings near tens of nanoseconds include clock overhead; absolute
savings are small at 100 audio blocks/sec. No acoustic latency or whole-app CPU
percentage improvement is claimed. Device callback/mixer behavior is unchanged.

## Validation and limits

Permanent tests cover complete/truncated packets, bounded header mutations, invalid
clock/sequence atomicity, reused poisoned storage, signed extrema/endian layout,
stereo boundaries, discontinuities, random sample patterns, partial integer offsets
and fractional drift against the retained scalar oracle. New API tests fail to
compile before implementation and pass afterward; this item is an optimization,
not an existing behavioral bug fix. All existing assertions remain.

215 common library tests pass; focused coverage runs pass 40 common audio and 11
host audio tests. Separate staging tests enforce zero steady allocations. Windows
host/GUI test targets cross-check; the installed offline SDK checks every native
host target with all features. The retained camera example integration was corrected
under T816. Formatting and changed-function complexity checks pass. Default Clippy completes;
strict Clippy still reports existing lint patterns, recorded as T842 with exact
diagnostics (no blanket allows added).

All 42 measured functions in the audio wire/queue/protocol scope and exercised host
adapters meet 80% executable-line coverage and CC≤9. The full inventory also retains
five unchanged host entry/lifecycle functions unmeasured by this focused run; it
must not be read as whole-file or whole-project acceptance. The full LLVM export
contains an unrelated unsigned counter underflow in `retirement.rs::reap` (T841).
Strict full-export ingestion fails; no counter was clamped or invented. The audio
report selects complete records for exactly the four stated files and preserves
all counters in those records. Both full and scoped exports are archived.

[Evidence and exact medians](artifacts/2026-10-10-t818/).
Reproduce stage replays with `BLENT_T818_MEASURE=1 cargo test --offline --locked
-p blent --release --test audio_staging -- --nocapture` and the corresponding
`-p blent-config --test audio_render_bench`. Historical before/after executables
were alternated on the same machine; current render tests retain exact samples.

</details>
