# T1186: trace retained-display pause boundaries

Diagnostics record original pause requests, acknowledgements and sampled native grab boundaries. Native and physical checks passed; the logs aid future attribution without recovering missing history or proving kernel retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T1184 incident logs show DPMS changes and stalled display commits, but omit the
original capture-control state and native grab progress. This change supplies
future evidence; it does not reconstruct missing historical facts or assert a
second root-cause repair. T1185's native completion fix remains in place.

## Implemented diagnostic contract

The helper writes to its existing stderr, tagged by the journal's helper process:

- `capture-activity request sequence=N active=0|1` follows successful decoding
  and monotonic sequence validation, before the capture transition is applied.
- `capture-activity ack sequence=N active=0|1` follows successful ACK transmission.
  Pending writer retirement or socket backpressure cannot manufacture this record.
  Repeated polls after ACK do not duplicate it. A paused ACK covers media
  quiescence; it does not release the connected native output or its buffers.
- `display-progress begin sequence=N` precedes a sampled paused GRABPIX operation.
  `display-progress end sequence=N elapsed_us=M` follows its synchronous return.
  Elapsed time brackets the diagnostic and native operation, including formatting
  and stderr write overhead; it is not a pure readback benchmark. Sampling logs
  the first admitted grab after each pause request, then at most once per five
  seconds. The underlying native service remains at most 10Hz.

Use original helper PID/start identity together with sequence; sequences alone
are not global identities. A missing sampled end record narrows an investigation,
but cannot exclude lost logs or blocked logging. These records do not expose
pixels, credentials or input events. They do not identify a DRM flip token,
certify imported-buffer synchronization or replace kernel retirement evidence.
Stdout, control bytes, ACK admission, frame publication and existing native cadence
are unchanged. Existing stderr remains the logging sink and retains its original
blocking/failure behavior; no detached logging queue or unbounded buffer is added.

## Permanent regressions and measured scope

Normal Cargo `evdi_helper` includes T1186 trace and admission regressions. The
trace case holds the original writer lease while pausing, observes native progress
before release, then requires the original ACK after actual writer release. An
independent native-return marker must lie between sampled begin/end records.
Additional calls within five seconds remain unsampled; a new pause sequence
immediately re-enables the first sample. No request/ACK record may duplicate.

The final unchanged trace test fails against an isolated archive of the original
production files, then passes on the implementation. Both initial and final red
logs are retained. The admission fixture exhausts all 32 combinations of active,
mode-present, failed, registered and running state, then examines each millisecond
from 0 through 1000. Only the valid paused state grabs, exactly at 100ms intervals;
media conversions, publications, update requests and disconnect remain absent.
The terminal uint64 sequence is retained without wrap or fresh protocol admission.

The complete offline, locked native collection passes **84 tests**: 69 helper,
12 conversion and one each capture_activity, evdi_modules and frame_retirement.
Existing sanitizer/race configurations remain enabled. Native GCC/gcov13.3.0
reports **100% executable-line coverage individually for all six changed
production functions**. All 235 production functions present in these collected
reports meet 80%; this is not a whole-project/platform coverage inventory. One
unused temporary fixture executable has no runtime profile; its zero report is
retained without replacing counters from executed fixtures.

Clang18.1.3 provides syntax validation and byte-accurate function boundaries for
local authored decision/operator counts. Changed production estimates are 2–8;
new tests and modified native dispatcher are 1–6. No function exceeds 9. These
estimates are not native SonarQube measurements. T1019 retains missing native
CFamily evidence and missing tree_sitter grammar dependencies; no tool download
was attempted. rustfmt and `git diff --check` pass.

The existing permanent stock-DRM guest fixture also passes with the exact staged
helper: active flip 17ms, paused flip 100ms, blocking modeset 0ms, DPMS off/on 99/100ms.
Request/ACK sequences 1–3 and paused begin/end sequence 2 appear in native logs.
The original helper is waited/reaped with status 0; its DRM owner closes, the
original EVDI device/module retires, and the VM exits. No flip/commit timeout is
observed. The guest uses cached kernel7.0.0-34-generic without Cinnamon, AMDGPU,
imports, host GPU passthrough or networking. This does not replace physical -38
pause/resume/wake acceptance under T1184.

## Ownership and installation

The event loop exclusively owns a two-scalar diagnostic cadence record inside
its existing capture context. It resets on pause admission and initial paused
loop start. Each diagnostic synchronously borrows that context/control and local
scalar arguments; no pointer or callback escapes, and no pixel allocation,
frame lease, worker, descriptor or mapping is added. Rejection and cutoff do
not admit diagnostic/native work. Original writer and native display ownership
remain distinct. Tests close original sockets and release the writer before
freeing buffers, clear all callback hooks before fixture lifetime ends, and wait
for native child/device retirement before resource reuse. Sanitizer results are
supporting evidence, not unrestricted native lifetime or kernel leak proof.

The built bundle is `target/t1186/evdi_helper` plus unchanged sibling libevdi.
The helper was backed up and atomically installed without restarting Blent.
`installation.json` records installed SHA256
`42617137d7a73df7d25bf32a1f831a3256a7fbfd1c9cdec9dd115a010b5dac5c`.
At the installation checkpoint, running helper PID 59391, start tick 448946, retained T1185 binary SHA256
`9c4b48c66274d1936f0426820c4be883f47ca25b3645289999beca3045326475`.
Capture/render progress continued without interruption. Diagnostics required
a subsequent service start; that later approved activation and physical result
are recorded below. The old executable inode stays owned by its original
running process until actual retirement; it was not overwritten in place.
Backup: `/backups/disk2/blent-work/2026-10-20-t1186/deployment/evdi_helper.before`.
No driver/library update, forced power transition, package transfer or push ran.

For recurrence evidence, read the existing service and kernel journals for the
same boot/time window. Correlate original helper process, capture request/ACK,
sampled native begin/end and EVDI commit timeouts. Preserve logs before any
recovery/restart. Do not infer historical T1184 state from a later successful run.


## Subsequent physical activation

On October 20 the owner approved a diagnostic restart and one brief display-off/wake
cycle. The installed diagnostic helper is now active as PID 95421. The original
Cinnamon/AMDGPU/kernel-38 target passes pause/native-service/resume, compositor
responsiveness and continuing tablet render ACK checks; the owner confirms
“Both working normally”. [Retained evidence](../2026-10-20-t1184-acceptance/README.md)
closes T1184's native acceptance, while preserving historical attribution limits
and the separate T914/T748/T749/T750 scopes.

</details>
