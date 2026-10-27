# Host performance and resource lifecycle review

Host probes reproduced blocking controller destruction, stale status after worker panic and uncapped command output. Source review identified separate supervision and allocation work. Temporary probes established findings, not permanent regressions or physical-platform acceptance.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Review of owned code at `bb1648e80e51fcc4eb030336cae35b88473d7fa3`, 2026-10-09.
Production code unchanged. Offline fault injection uses disposable processes,
cached dependencies and fake sessions; no installed service, sensor, driver or
external dependency was modified. See the companion [Android lifecycle review](2026-10-09-resource-lifecycle.md)
and [project boundary review](2026-10-09-project-boundaries.md).

## Reproduced owner behavior

The [replay](artifacts/2026-10-09-host-review/replay.py) compiles unchanged
`host/src/audio_control.rs` and `camera_control.rs`, uses current common contracts
and seeds its offline build with the workspace lockfile. [Results](artifacts/2026-10-09-host-review/results.json)
record source hashes and tool identity. These are review probes, not permanent
normal-suite regressions or physical platform acceptance.

| Injection | Observed behavior | Finding |
| --- | --- | --- |
| Audio session held on a cooperative latch during controller destruction | Drop remains blocked beyond 200 ms; completes after latch release | T811 |
| Camera session held identically | Same synchronous destruction wait | T811 |
| Audio session panics in its controller thread | Status remains `Starting`; another Start fails with `Stop audio before changing an active session` | T812 |
| Camera session panics in its controller thread | Status remains `Starting` | T812 |
| Local command emits 4 MiB stdout and 4 MiB stderr within a two-second limit | Both complete streams are retained and returned | T815 |

The 200 ms observation is a lower bound, not a configured shutdown timeout.
All held sessions were released and threads joined. The output fixture establishes
absence of a small output cap, not an observed OOM. Worker panics were injected;
this does not establish a naturally occurring panic in a native session.

## Lifecycle findings

- **T811: controller destruction can stall application exit.** Both controllers
  synchronously `join` their worker in Drop. Their cancellation arms signal Stop
  then await the session without a deadline. GUI panels own these controllers,
  so a stalled session can block window/application teardown. Give controllers
  an explicit asynchronous shutdown outcome and bounded control-side retirement;
  retain native ownership until safely reaped. Do not fix this by detaching an
  unlimited number of live workers or freeing resources still in use. Add a
  permanent held-cleanup regression that verifies UI progress, bounded pending
  work, no overlapping restart and eventual retirement after release.
- **T812: controller worker death leaves stale status/admission.** Top-level
  thread completion/panic is not converted into an owner outcome. Observe worker
  termination outside the session future, publish failure and distinguish a dead
  controller from a retryable session. Preserve explicit user Start and confirmed
  retirement; do not clear admission while native ownership is uncertain. Add
  permanent panic/early-exit tests and verify Stop/retry status. Rust unwind
  handling is not containment of aborts or native faults.
- **T813: session workers have no aggregate health supervision.**
  `session_core::spawn_server` logs errors and erases results into `JoinHandle<()>`;
  `session.rs::LinuxCapture::start` does the same for capture failures. Runtime
  stores handles but observes them only during shutdown. Thus terminal listener
  or capture failure need not retire its sibling work or publish a failed session.
  This is a source-level finding, separate from the controller panic probe.
  Preserve typed outcomes and supervise unexpected completion, retiring only the
  affected session. Test fake listener/capture Err, panic and early success;
  verify attachment/authentication invalidation and unrelated session progress.
- **T814: a graceful deadline is not a whole-shutdown deadline.** Audio process
  retirement waits 500 ms, then awaits `kill()` without another bound; camera
  retirement waits one second, then awaits kill/wait without another bound.
  Common async command timeout also awaits kill afterward. Session Runtime
  aborts after five seconds but then awaits task destruction without a new bound.
  Persistence shutdown awaits a committing filesystem worker, which cancellation
  cannot interrupt. A nonyielding native call or stuck filesystem can therefore
  outlive the advertised/control-side budget. No unkillable process or filesystem
  hang was induced. Define separate grace, kill-request, reap and owner-response
  budgets with explicit incomplete outcomes and bounded quarantine. Keep atomic
  config commit semantics. `capture/process.rs` already bounds its post-kill wait,
  but discards whether exit was confirmed; preserve that distinction too.
  Permanent fakes must hold each phase independently and verify continued control
  progress and no premature release/replacement.
- **T815: command output is time-bounded but not byte-bounded.**
  `common/src/commands.rs::CapturedOutput` spools stdout/stderr to temporary files,
  then calls `read_to_end` on both. In the async path this synchronous collection
  runs after the timed future completes. A fast/noisy command can consume disk,
  memory and event-thread time independently of the command deadline. Add
  configurable per-stream/total budgets enforced during collection, bounded
  draining and cancellation/reaping. Reuse the ownership lessons from
  `host/src/command_output.rs`; do not reintroduce inherited-pipe hangs.
  Permanent tests need exact/over limits, both streams, descendant-held outputs,
  slow collection, partial reads and cleanup on every error.
- **T819: optional in-process encoding shares daemon failure fate.** The default
  FFmpeg and C capture helpers provide process boundaries. `inproc-encoder`
  instead runs libavcodec in a started `spawn_blocking` task; abort cannot stop
  such a call, and native faults affect the daemon. Evaluate a private encoder
  worker using the existing bounded shared-memory lease protocol. Compare copy
  savings, startup, RSS and failure recovery against both current paths. This
  must preserve final-reference lifetime and generation invalidation, not undo
  T787's lease work. Native worker death/hang tests belong in disposable fixtures.
  No claim of recovery from a system-wide GPU/kernel fault.

## Performance opportunities, not measured speedups

| Finding | Current source work | Candidate and required evidence |
| --- | --- | --- |
| T816 | `camera/decoder.rs::transfer` allocates and zeroes a whole YUV420 frame on every decode, then publishes `Arc<Vec<u8>>` through a latest-value watch | Small bounded reusable frame leases. Validate slow consumers, partial reads, EOF, cancellation, generation changes and immutable outstanding previews before measuring allocation and latency. |
| T818 | Audio packet input constructs a combined packet, decodes into fixed PCM storage, then allocates/copies native output. Speaker capture/serialization adds staging; identity-rate render still interpolates samples | Reusable per-owner byte buffers, validated payload views and exact identity-rate bulk copy. Measure incremental benefit; preserve atomic sequence validation, clock/discontinuity, phase, endian and stereo semantics. |

At 1280×720 YUV420, one camera frame is 1,382,400 bytes; 30 FPS allocates
41,472,000 payload bytes/s. At 1920×1080 it is 93,312,000 bytes/s. These are
source-derived allocation-volume estimates, **not measured RSS, bandwidth or
time savings**. The watch channel is already bounded; this is allocation churn,
not evidence of an unbounded frame queue. A pool must include leases held by
both output and preview, reject/coalesce when full, and never overwrite readers.

Native audio already uses fixed-size PCM blocks and bounded queues; its callback
uses `try_lock` with silence on contention. Do not replace this with blocking
locking or assert heap allocation inside that callback. Before custom SIMD,
remove duplicate fill/serialization and use contiguous copy on the exact identity
path. Nonidentity interpolation still needs exact rounding and phase tests.

## Data structures, scalability and scope

Keep host watch channels for latest settings/camera frames and bounded audio/video
storage. The device scheduler uses a HashMap for active work and a VecDeque for
queued work; active polling is limited to its configured workers. No measured
case here justifies a tree, extra index or lock-free rewrite. Existing C dirty
bitmaps/two-span histories, weighted worker partitioning and AVX2/portable kernels
remain preferable to blanket per-frame dynamic structures. Their historical
measurements are in the [C review](2026-10-09-c-performance.md); they were not rerun
or presented as new measurements in this review.

Review covered host session/control ownership, transport, camera decode/output,
audio framing/native queues, capture subprocess/in-process ownership, command
collection, persistence, GUI adapters and existing C/GPU boundaries. New boundary
findings and Wi-Fi/GUI blocking I/O are in the companion project review. This is
not a new whole-repository sanitizer, fuzz, native Windows or per-function coverage
certification. Existing scope exclusions and physical acceptance items remain.

Implementation order: independent contracts where required; bounded owner
retirement and terminal outcomes; output/I/O budgets; then allocation and hardware
experiments. Each behavioral fix needs its permanent failing regression before
the implementation, passing regression afterward and individual coverage.

</details>
