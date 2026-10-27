# T794: mapped VAAPI producer implementation plan

The mapped-surface encoder candidate was rejected after repeatable CPU regressions. This document preserves the earlier proposal, independent ownership contracts and qualification criteria, including requirements to keep existing supported encoder choices. Its planning-stage open status and proposed thresholds are historical; it is neither a shipped path nor a renewed implementation commitment.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Disposition (2026-10-10): the tested candidate was rejected after repeatable CPU
regressions; see [native evidence](artifacts/2026-10-10-t794/README.md). The plan
below records the earlier proposal and qualification criteria, not an open
implementation commitment or shipped mapped path.

Planning baseline: `3dea8f2`, 2026-10-10. No implementation or new performance
measurement accompanies this plan. T794 remains open. Work uses cached tools and
stock dependencies; no stock EVDI driver/library changes, downloads, cross-device admission or automatic
change to the installed encoder path.

Owner clarification: retain the current implementation for users without AMD or
without mapped-surface support. This is an additive optimization selected from
the actual GPU/backend capabilities, never an AMD-vendor gate. Preserve existing
VAAPI upload, default FFmpeg CLI, software encoding and other supported encoder
backends. Intel/NVIDIA users retain their existing paths; available stock VAAPI
implementations may qualify through the same probes. Native acceptance on two AMD
nodes is evidence for those devices only, not a claim of all-GPU compatibility.

The proposed benefit is removing the explicit CPU-NV12-to-VAAPI upload. EVDI
readback, CPU color conversion, GPU encoding and compressed-packet transport
remain. CPU stores to mapped GPU memory still move data and may be slower than
stores to cached system RAM. Call this a mapped-surface path until measurements
establish what it saves; do not claim end-to-end zero-copy.

## What existing evidence establishes

- `host/evdi/capture.c::publish_shared_capture` converts directly into leased
  memfd slots. `raw_ring.c` keeps dirty history separately for each slot, and
  retains pending capture when every slot is leased.
- `conversion.h::conv_job_t`, native/scaled kernels in `conversion.c`, and the
  reduced job in `conversion_box.h` address destination rows using output width.
  They need separate Y/UV strides and bounded plane extents before mapped output.
- `common/src/raw_frame.rs` describes packed CPU memory. Its Linux adapters
  depend on sealed memfd geometry and shared atomic slot states. A GPU descriptor
  is a different contract; it cannot be substituted for that memfd.
- `encoder_hardware.rs::Frames::upload` gets a hardware frame and calls
  `av_hwframe_transfer_data`. The codec owns references to this uploaded frame.
- T819 moved the optional native encoder into a supervised child. The C producer
  and codec live in separate processes. A mapped pointer, `AVFrame *`, or
  `VASurfaceID` alone is not a transferable surface capability.
- T787's [mapping probe](artifacts/2026-10-09-t787/mapping-probe.c) uploads a frame,
  maps it with `WRITE | OVERWRITE | DIRECT`, records status/pitch, then frees it.
  Both local AMD nodes accepted that map at 1280×800, pitch 1280. The probe does
  **not** write mapped pixels, preserve sparse history, export/import the surface,
  or encode the mapped result. `OVERWRITE` must not be treated as preserving pixels.
- Historical T787 FIFO/shared boundary medians were 3.836/3.177 ms p50 and
  4.644/3.542 ms p95; a separate resource replay measured 595.066/433.433 ms CPU
  per 150 frames. These [measurements](artifacts/2026-10-09-t787/README.md) include
  upload on both paths and predate T819 production isolation. They do not predict
  T794's gain. Rebuild a current isolated-worker baseline for comparison.

## Architecture choice and first decision gate

Prefer **producer-owned surfaces with same-device descriptor transfer**: the C
producer owns a bounded GPU pool, maps one exclusively, runs existing conversion,
unmaps, and publishes a transferable descriptor/lease. The isolated codec child
imports that same storage and submits it without `av_hwframe_transfer_data`.
Parent orchestration supervises both children and carries no native GPU objects.

An isolated stock-API probe must establish whether this is supported before
production integration. Candidate adapters use stock VAAPI/FFmpeg DRM PRIME
export/import APIs and Unix descriptor passing. Exact supported flags, modifiers,
imported frame context and completion semantics must be verified against the
locally installed headers/implementation and patterned execution. Successful
mapping alone does not establish successful import or safe reuse.

Evaluate alternatives at that gate:

| Candidate | Tradeoff | Decision |
|---|---|---|
| Producer-owned map, same-device export/import | Keeps producer/codec process ownership and avoids CPU raw transport; needs validated import, synchronization and finite surface lifetimes | Preferred prototype |
| Codec-owned pool exported to producer for CPU mapping | Fits codec context ownership but reverses pool negotiation; still needs transferable descriptors and validated mapping/synchronization | Alternate probe if stock API constraints favor it |
| Conversion and codec inside one child | Allows local pointers, but moving RGB capture snapshots adds transport/ownership work; merging EVDI capture with encoding couples capture to codec stalls | Do not silently adopt as fallback; separate architecture decision if descriptor approach fails |
| Existing CPU memfd plus upload | Already validated, preserves current ownership and sparse conversion | Production fallback and paired baseline |

Same-device import of newly owned, explicitly described surfaces is distinct from
T578's cross-device X11 pixmap import. Retain same-node identity checks. Reuse
validated identity checks, not the existing GPU path's implicit-layout assumptions.
No dependency on unvalidated DRI3 export or cross-device support is introduced.

## Contracts before native integration

Keep layout validation, state transitions and capability policy independent of
libavcodec, libva, EVDI, pthreads and OS descriptors. Use focused C pixel views and
portable Rust schema/state types; translate through explicit Linux adapters.
Reuse T823's opaque-owner pattern and T819's supervision, generation and nonce
handling. Do not expose native owner structs or add GPU fields to CPU memfd layouts.

1. **Writable CPU view.** Dimensions, pixel order, scale, Y/UV pointers, independent
   positive strides, and accessible byte lengths. Validate each plane with checked
   `(rows - 1) * stride + visible_width`, integer conversions and nonoverlap before
   dispatch. Preserve even chroma alignment and current dimension limits. Reject
   negative/short/overflowing strides; don't infer UV from `Y + width * height`.
   Conversion threads borrow the view only until their joined completion. SIMD
   and box kernels use destination strides without changing arithmetic/rounding.
2. **Surface identity and transport.** Version, format, dimensions, color metadata,
   pool generation, session nonce, slot, sequence and capture timestamp. Keep
   native object/plane/modifier descriptions in the Linux transfer adapter with
   fixed object/plane/FD bounds, checked indices/offsets/pitches/sizes, and exact
   expected descriptor count. Reject unknown layouts/modifiers and mismatched
   devices. GPU tiling metadata is not a linear CPU view. Derive the latter only
   from a successful map. Never serialize addresses or native surface IDs as
   transferable authority. C/Rust golden vectors check schema agreement.
3. **Lifetime.** `Free`, `Writing`, `Ready`, `Reading`, `Retiring`, and failed states
   have one owner and explicit transitions. Only `Writing` exposes mutable pixels.
   Publish after conversion joins and mapping writes become visible. Stop CPU
   access before codec submission. Recycle only after both final codec-reference
   retirement and demonstrated device completion; packet delivery alone is not
   completion. Probe stock synchronization on imported surfaces. A native wait
   must stay in a supervised child, never on parent event-loop or teardown paths.
4. **Release protocol.** Acknowledgments are bounded and generation/nonce/sequence
   checked; duplicate, stale or forged releases cannot authorize writes. An
   acknowledgment corresponds to actual retirement plus GPU completion, not
   receipt of a descriptor. Avoid allocation/waiting in FFmpeg final-reference
   callbacks: record retirement in bounded storage, finish native synchronization
   through the owning adapter. Close received descriptors on every reject path.
5. **Capacity.** Reuse user-controlled 2–8 slot policy, default four, if the native
   capability check supports it; otherwise reject that candidate configuration.
   Preallocate a finite pool and import/cache descriptors per slot/generation.
   Do not let libavutil dynamically growing pools evade the application bound.
   Set a checked byte budget from actual allocations and bound retired generations
   too. Full pool retains latest pending damage, rather than allocating or queuing
   unlimited frames. Configuration documents memory/latency tradeoffs.

## Sparse conversion and recovery

Dirty history belongs to each physical surface, as current raw-ring history belongs
to each memfd slot. Accumulate all intervening damage while a surface is busy.
First use, resize, scale/pixel-order change, uncertain contents, allocation or
generation change require a complete refresh. Coalesce damage with existing bounded
row spans and adaptive/full conversion policy; do not add an unbounded rect list.

First native prototype always fills the full visible image, using overwrite mapping
only in that case. Sparse mode requires a separate successful preservation probe
without overwrite/discard semantics. Rotate at least three surfaces through
different disjoint changes and verify the entire result after each reuse. If the
driver cannot preserve mapped content safely, benchmark full conversion versus
current sparse CPU conversion plus upload. Do not accept sparse-workload regressions
merely because upload disappeared. Clear history only after successful conversion
and synchronization; uncertain failures invalidate the surface. A failed publish
retains pending capture, with state/history consistent for the next attempt.

On unsupported map/export/import/layout, initialize the existing upload path.
Expose selected path and fallback reason through capabilities/diagnostics. Keep
initial selection explicit; automatic VAAPI admission remains unchanged until
measured policy and native acceptance justify a separate change.

Keep policy independent of VAAPI: request the existing encoder/backend first, then
optionally negotiate a mapped-input capability with its native adapter. Unsupported
feature builds, other GPU backends, unvalidated devices or declined probing retain
their previously selected path. Do not silently change an NVENC/software request
into VAAPI or make direct-map support a condition for starting a session. Preserve
the established CLI route as well as optional-native upload; test these fallback
choices with capabilities faked independently of vendor strings. A failure of the
new path must not remove support that the unchanged path previously provided.

On device loss, worker death, map/submit timeout or failed completion, quarantine
the affected generation. The supervisor retires children and invalidates pending
packets before rebuilding an upload session with fresh nonce, codec configuration,
full capture and IDR. Never reclaim a suspect surface merely because a timeout
expired or a task was cancelled. Bound restart attempts and retained resources;
if a kernel-stuck process cannot be reaped, stop admission instead of accumulating
replacement pools. Process isolation cannot guarantee recovery from a GPU/kernel
hang. Invalidate history when fallback changes output storage.

## Implementation stages in dependency order

1. **Stock native feasibility, disposable processes.** Extend the local probe to
   write patterned mapped NV12, unmap, export, pass FDs to another process, import,
   encode/decode, await completion and reuse. Test both local nodes independently,
   1280×800 and padded 1282×802, full overwrite and preserved sparse updates.
   Include bounded operation deadlines/cleanup. Save build provenance and raw
   results. Stop this design if sharing/completion is unsupported; keep upload.
2. **Portable contracts and stride-aware conversion.** Add T794 tests before
   changes: padded unequal Y/UV strides, guard bytes, separate plane allocations,
   scalar/AVX2 parity, scales 1–4, pixel orders and sparse histories. Extend the
   contract and all kernels/callers together. Packed FIFO/memfd behavior stays
   byte-identical. Define the finite pool state machine and transfer golden vectors.
3. **Linux producer/consumer adapters.** Implement opaque surface ownership,
   map/unmap, export/import, bounded descriptor transfer and completion. Supply
   fake native seams for permanent error/lifetime tests. Keep C capture event
   handling, pure conversion, transport and GPU operations in cohesive modules.
4. **Isolated worker and session integration.** Add explicit capability selection
   to T819 worker jobs through independently defined settings. Compose adapter
   selection at capture startup; exercise restart, fallback, IDR, slot exhaustion,
   parent/peer death and cancellation before allowing real capture admission.
5. **Sparse history and performance qualification.** Enable preserved sparse
   mapping only where stage 1 proved it. Run paired current-source tests below,
   then one ordinary host/Android correctness smoke on supported hardware. This
   does not reopen the declined multi-tablet/large-machine campaign.
6. **Disposition.** Ship only supported measured improvements with documented
   scope and fallback, or record evidence-backed rejection. Remove T794 only
   when its acceptance/rejection evidence is complete. Missing native validation
   remains explicit and unresolved. Implementation belongs in one T794 commit
   after internal stages pass; this plan is not that implementation commit.

## Permanent tests and gates

- Put behavioral regressions in normal C/Rust suites, link names to T794, record
  failure before changing behavior, then rerun unchanged tests successfully.
  Existing conversion, raw lease and T819 regressions remain permanently retained.
- Bounded property/fuzz cases cover zero/odd/maximum/overflow dimensions; invalid
  strides, offsets, extents, alignment, planes, object indices, modifiers and FD
  counts; truncated/extra messages; stale generations; sequence overflow;
  duplicate releases; wrong devices; overlapping output; invalid damage spans.
  Rejection must precede any unsafe address calculation, native dispatch or write.
- Model delayed final references, map/unmap/export/import failures, submission
  failures, EAGAIN, lost completion, allocation failure at every ownership step,
  pool exhaustion, resized busy surfaces and repeated restart. Verify exact
  descriptor/reference cleanup, no premature reuse, bounded memory and supervisor
  responsiveness. Inject device loss; do not require physical GPU removal.
- Native tests compare full mapped plane readback against current conversion,
  including untouched regions and padding guards. Encode/decode patterned frames,
  color metadata, scaling, changing scenes and forced IDRs. Compare exact encoded
  bytes where deterministic; otherwise predefine decoded-pixel equivalence against
  the same baseline/settings and retain raw output. Successful API calls alone
  are insufficient. Tests must distinguish unavailable hardware from passing.
- ASan/UBSan cover project C code; retain meaningful concurrency/lifetime tests
  across process boundaries. Measure every changed/new maintained production
  function with native counters: at least 80% executable-line coverage, including
  separately compiled callbacks. Missing native counters are not a pass. Keep
  cyclomatic complexity ≤9 and enforce independent public contracts in existing
  architecture tests. No exclusions, test weakening or dependency instrumentation.

## Paired measurement and acceptance proposal

Use current production isolation on both sides: CPU memfd plus upload versus
producer-mapped surface. Fix source revision, stock SDK/driver/device, encoder
profile, size, scale, conversion workers, slot count and input sequence. Alternate
at least five measured pairs after warmup, without simultaneous benchmark load.
Include full motion, sparse/disjoint updates, idle refresh and rotating held slots;
1280×800, padded 1282×802 and 1920×1080; packed CPU baseline and identical color math.

Record conversion, map/unmap/synchronization, descriptor transfer, import (startup
and steady), upload baseline, codec submission and packet receipt separately, plus
capture-publication-to-parent-packet p50/p95/p99. Count producer+codec+parent CPU,
allocation rate, bytes copied, slot stalls/drops, startup/restart latency, FD count,
PSS where available and reported GPU allocation bytes. RSS sums double-count
shared pages; unavailable GPU accounting must be stated. Report actual skipped
upload calls, not an inferred byte count presented as measured bandwidth.

Proposed shipping threshold: repeatable ≥5% reduction in total pipeline CPU or
p95 publication-to-packet latency across the intended admitted workload, beyond
observed run-to-run noise, with no repeatable >5% regression in the other metric,
full/sparse workloads, p99 or startup. This is a proposed engineering gate, not an
existing owner requirement. Correctness, finite resource use and contained failures
are mandatory independent of speed. Report any memory increase in actual bytes
and tie it to configured capacity; don't claim fewer pools implies lower memory.
Reject a noisy/no-gain result or a workload regression; leave upload as default.
A workload/device-specific admission rule needs its own measured evidence and
bounded policy rather than a blanket enablement based on these two AMD nodes.

</details>
