# Android performance review, 2026-10-09

This static Android review used focused host-JVM probes to identify UI blocking, unbounded response memory and allocation opportunities. Proposed savings were stage measurements, not ART, battery or whole-application gains. Later implementation reports supply the fix and native evidence.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Review of owned Android code at `bb1648e`, including video, camera, audio,
input/control, lifecycle, UI, allocation bounds and startup metadata. The source
inventory contains 75 production Kotlin files. This is a static pipeline review
with focused host-JVM probes, not a fresh ART profile or a claim that every method
has been benchmarked. No application behavior, installed packages, services,
dependencies or device settings changed. No downloads or sensor activation.

Prioritize UI retirement latency and bounded update-response memory, then the
small, straightforward audio savings. Hardware selection and native audio
scheduling warrant measurement; there is no evidence of a new across-the-board
FPS improvement. Findings T795–T805 are in [TODO.md](../../TODO.md).

## New local measurements

[Raw results](artifacts/2026-10-09-android-performance/results.json) include source
hashes, compiler hashes, runtime and all five alternating trials. Run the
[offline replay](artifacts/2026-10-09-android-performance/replay.py) with Python 3;
it refuses missing compiler jars instead of downloading them. Proposed changes
exist only in temporary benchmark copies. Production files remain untouched.

| Host-JVM workload | Current median | Temporary candidate median | Interpretation |
| --- | ---: | ---: | --- |
| Stereo queue offer + poll, identity rate | 1.614 µs; 2,048.9 allocated B/block | 0.037 µs; 64.9 B/block | Borrow the removed block's immutable PCM array when rate, phase and offset permit it. |
| Drift correction, unchanged valid completed windows | 69.7 ns; 520.0 allocated B/call | 9.7 ns; 16.0 B/call | Cache the derived rational result after checking both windows remain valid. |
| Codec retirement with a borrowed operation held | 500.12 ms caller wait | No replacement implementation tested | Real `CodecLifetime`, stub native codec, controlled latch. |

Queue figures cover 50,000 blocks/trial; drift figures cover 500,000 calls/trial.
Each process warms eight quarter-length passes. Allocation counters cover the
calling JVM thread, including setup amortized over each pass; they are not ART
heap or RSS measurements. The benchmark uses immutable prebuilt source PCM and
excludes sockets, native playback and capture. Five signed-rate, random-PCM
replays with epoch changes, discontinuities, overflow and expiry produce matching
checksums across current/candidate code. This supporting probe does not replace
permanent regression tests when implementing the changes.

At 100 blocks/s, the identity candidate avoids about **198 kB/s of JVM allocation**
in this replay, but saves only about **0.16 ms CPU per second** on this host.
Unchanged-window caching avoids about **50 kB/s** at 100 calls/s; its absolute CPU
saving is smaller still. Actual fast-path frequency, ART costs, GC and native
playback effects remain unmeasured. These are good cleanup candidates, not claims
of a 43× faster application or audible latency improvement.

## Findings and implementation conditions

| ID | Evidence and opportunity | Required validation before resolution |
| --- | --- | --- |
| T798 | `MainActivity.onStop` / Surface destruction synchronously reach `DecoderSession.releaseCodec`, which holds its monitor while joining native retirement for 500 ms. Native destruction already has its own thread and global admission gate. The existing T386 test allows 750 ms, so it does not protect UI responsiveness. | Make UI retirement return promptly while retaining closed admission, borrowed storage and replacement gating. Add a failing main-looper/latch regression first, then test delayed borrow, delayed destruction, recreation and stale callbacks. |
| T804 | `HttpReleaseCall.newer` calls `response.body.string()` then parses the whole JSON object without a byte limit. The 15 s timeout bounds time, not peak memory; even unknown-length successful responses are fully buffered. | Bound decoded response bytes before String/JSON allocation; preserve cancellation and closure. Add a synthetic oversized/chunked response regression, plus exact-boundary, valid, truncated and multibyte cases. Normal update checks are a cold path; no current OOM was reproduced. |
| T795 | `AudioPlaybackQueue.poll/interpolate` allocates PCM and a deque snapshot and performs 960 channel interpolations even for an aligned identity-rate block. The general path also repeats block lookup per channel/sample. | Use the measured identity shortcut first. Retain tests for array ownership, exact samples/timestamps/discontinuity, signed drift, fractional phase/offset transitions, wrap, age, overflow and invalid lengths. Consider a fixed ring or hoisted stereo indexing only after measuring the remaining nonidentity path. |
| T796 | `AudioClockDrift.correction` repeats BigInteger arithmetic on unchanged 2–5 s native-rate windows, ordinarily checked every audio output block. | Cache by both completed rates while preserving freshness checks, resets, epochs and exact 1000 ppm rejection. Test expiry without progress, source-only/destination-only updates, invalid snapshots, clear and overflow boundaries. Do not replace safe rational arithmetic with overflowing Long products. |
| T797 | Stereo receive allocates a 1,920 B packet payload and a 1,920 B ShortArray; output converts PCM back into a reusable 1,920 B ByteArray before `AudioTrack.write`. Headers, timestamp buffers and native `AudioTimestamp` instances add smaller churn. | Evaluate direct decode into owned PCM, reusable per-owner scratch and a typed PCM output contract; local SDK 34 exposes `AudioTrack.write(short[], …, WRITE_NON_BLOCKING)`. Retain units, stereo alignment, partial/zero/dead writes, byte order, cancellation and queued ownership. Measure parsing: a per-sample reader can lose to the existing bulk copy. No end-to-end zero-copy claim. |
| T799 | `VideoReceiver.awaitSurface` polls every 200 ms; `setSurface` records readiness without waking it. Readiness just after a poll adds nearly 200 ms of avoidable startup delay. | A generation-aware notification should preserve off-UI native startup and handle readiness-before-wait, destroy/recreate, stop and reconnect without lost wakeups or stale setup. Add deterministic scheduler regression. This affects startup, not steady FPS. |
| T801 | Microphone zero reads and speaker zero writes retry at 2 ms intervals. Each attempt also queries route/permission or recording configuration; paused rendering wakes every 5 ms and still queries the native clock. `AudioTrack` uses at least 1,920 native frames (40 ms at 48 kHz), independent of the app queue, and requests no performance mode. | Profile wakeups, native calls, underruns and actual buffer/performance mode. Evaluate event-driven state changes, safe query reuse, paced polling and an optional supported low-latency mode before considering AAudio/JNI. Preserve prompt route/privacy/focus detection and bounded Stop/stall behavior. Buffer reduction needs native acceptance; requesting a mode is not proof it was granted. |
| T802 | `DecoderCapabilities.report` enumerates `MediaCodecList` for each of four MIME types, then `MediaInventory.describe` queries capabilities again. Profile mapping/support checks/JSON string deduplication occur before the 32-entry output limit. | Reuse one per-report inventory/capability snapshot and deduplicate primitive profile keys before repeated checks; cache format support only with complete keys. Preserve ordering, caps, exact-format validation and unknown-vs-supported behavior with counted fake inventory tests. Do not stop scanning before determining supported entries or persist stale hardware facts. Cold-path gain unmeasured. |
| T803 | `ControlSession.enqueue` records queued bytes but accepts messages until the socket refuses them. Existing T387 stalled replay has an 8 KiB refusing fake, not a Blent application latency/byte budget. Slow drains can retain old touch/mouse events and render ACKs. | First reproduce delayed draining with a still-accepting fake, measure age/bytes and design a configurable conservative budget. Preserve handshake and ordered touch/mouse transitions; retire the whole uncertain gesture/session as today on rejection. No indiscriminate event dropping or stylus feature work. No live backlog was observed. |
| T805 | `CameraLink.exchange` writes one packet, waits for host decoder-input ACK, then releases the codec output buffer in `sendBuffer`'s finally. Serial cycles longer than 33.3 ms cannot sustain requested 30 FPS even when average bandwidth suffices. | Research only: sweep synthetic acceptance delays and record delivery age/held buffers before proposing a small bounded overlap. Extra workers or detached storage can increase latency and memory. Preserve one-in-flight default unless a measured candidate improves acceptance age without violating ordering, freshness, Stop or ownership. A protocol/host change would require explicit cross-boundary contracts and regressions. No current-camera throughput failure demonstrated. |

T795 and T796 are independently implementable. T797 should measure its incremental
benefit after T795 so allocation savings are not counted twice. T801 should compare
native scheduling after the inexpensive PCM work; it must retain the existing
T718/T720 physical-audio acceptance requirements. Other research can start locally
and is therefore open, rather than blocked merely because eventual target
acceptance is needed.

## Copies, large allocations and scaling

| Path | Current ownership / bound | Assessment |
| --- | --- | --- |
| Display compressed input | Reusable 512 KiB initial heap array, capped at 8 MiB + 1; borrowed synchronous dispatch then copy into codec input | Growth can temporarily retain old and new arrays until GC. No per-frame backing array in steady state. Shrinking every frame or preallocating the maximum would trade one cost for another without evidence. |
| Experimental callback decoder | At most two detached access units, each up to 8 MiB, in addition to reader storage | Roughly 24 MiB of payload storage at simultaneous maxima, excluding object/native overhead and transient reader growth. It is not the default path. |
| Display decoded output | MediaCodec output Surface, retained SurfaceView | No owned Kotlin full-frame RGB/YUV array or conversion kernel. Native codec/graphics buffer memory is not measured here. |
| Camera | Camera2 Surface into codec; bounded 2 MiB encoded packets; 8 KiB segment-sized staging to socket | Avoids application raw-pixel copies, but native acceleration and socket zero-copy are distinct questions. Preserve bounded staging and codec buffer lifetime. |
| Speaker queue | At most 20 × 960 shorts = 38,400 B PCM plus objects; 20–200 ms target | The allocation rate is more relevant than retained queue size. A ring would remove the O(Q) reference snapshot, Q ≤ 20, not change the necessary O(samples) interpolation. |
| Audio decode/convert | At least 5,760 B of new PCM backing arrays per stereo packet with interpolation, about 576 kB/s at 100/s | Source-level byte accounting excludes headers, wrappers and object overhead. T795 removes one array only when aligned identity conditions hold; T797 addresses other copies. The output staging array is reused, not allocated per block. |
| Frame timing / touch | Fixed 64-entry primitive history with validated direct cache; at most 10 touch slots | Small bounded fallbacks and scans are appropriate. No evidence supports replacing them with general maps or parallel workers. |
| Update metadata | Full successful response and JSON allocation, no explicit byte cap | T804 is the clearest unbounded application allocation found in this review. |

Nested stereo loops are 480 frames × 2 channels, not quadratic in image size.
Touch pointer/history iteration is proportional to samples delivered; those
samples have ordering semantics. Codec/profile nesting is finite startup work,
with repeated capabilities addressed by T802. Compose samples diagnostics at
one-second intervals only while connected and statistics are enabled; video
pixels do not traverse Compose state each frame.

## Hardware offload, SIMD and alternatives

The display already delegates decoding to MediaCodec and output to a Surface;
selection prefers advertised hardware unless an explicit supported selection
chooses otherwise. Camera feeds a codec Surface directly using stock encoder
selection. Speech microphone mode requests the
platform AEC when available. Availability/enabled state does not establish DSP
execution or prove acoustic correctness.

No owned Android pixel loop currently warrants a GPU/NEON rewrite. The remaining
audio arithmetic is only 480-frame blocks; transferring it to GPU compute or
adding JNI/native lifetime complexity has no measured benefit. Remove redundant
identity interpolation and PCM serialization first. If native profiling later
finds nonidentity interpolation dominant, compare hoisted scalar stereo indexing,
bulk conversion and a bounded SIMD kernel with exact signed rounding and tail
tests under T795. SIMD must not weaken fractional clock correction.

AudioTrack's local SDK exposes performance-mode and offloaded-playback APIs, but
their existence does not demonstrate a supported low-latency offload path for
Blent's live PCM plus application drift correction. T801 starts with the existing
adapter and reports actual granted state. An AAudio/Oboe migration is a larger
native-backend experiment, not a guaranteed zero-copy or lower-latency switch;
no library was fetched. Larger buffers reduce some wakeups but add interactive
lag, so capacity remains user-controlled and measured.

Retain historical decisions unless new workload evidence justifies revisiting:

- [T403 direct compressed input](../benchmarks/2026-09-18-decoder-input.md)
  eliminated an application copy but changed motion p99 from 34.54 to 34.68 ms;
  it did not establish a gain.
- [T386 callback profiles](../benchmarks/2026-09-18-decoder-profiles.md)
  raised motion allocations and worsened tail latency. Hardware codecs already
  run asynchronously internally; adding an application worker pool does not
  partition one dependent compressed stream into useful independent work.
- [T607 camera chunks](2026-09-26-camera-chunks.md) found no useful ADB throughput
  improvement above 8 KiB despite synthetic sink-call improvements. No reason
  here to reopen chunk sizing or equate method calls with transport syscalls.

No new thermal, battery, hardware-codec utilization, concurrent camera/audio/video
capacity, ART GC or physical presentation measurements were collected. Existing
native acceptance blockers and explicitly declined scopes remain unchanged.

</details>
