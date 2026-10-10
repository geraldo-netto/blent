# Host power following and unchanged frames

Owner decision (2026-10-10): host lock or display-off pauses display sharing only;
active webcam/audio continue. Host suspend/shutdown necessarily interrupts all
resources. Resume should restore enabled resources without restoring stale
connection authority. Implementation is tracked under T913–T915 in TODO.md.

## Current implementation, verified locally

- `host/evdi/capture.c::on_dpms` records `dpms_on` and logs it. No production
  capture decision reads that flag; it also appears in diagnostic statistics.
- No host suspend/resume event adapter was found in maintained host sources.
- `host/evdi/writer.c::writer_frame_due` skips redundant motion-rate output but
  repeats the current frame at a default 200 ms interval (5 Hz). A valid local
  adaptive-idle lease can increase the interval to 500 ms (2 Hz).
- `common/src/idle.rs` and `host/src/capture/idle.rs` already gate sparse cadence
  using matched encoder timestamps/render ACKs. `adaptive_idle` defaults false.
  This is existing implementation, not proof of current battery savings or
  physical acceptance; T492/T679 remain unresolved.
- The optional in-process path in
  `capture.c::{publish_shared_capture,shared_poll_timeout}` retains 200 ms idle
  output. It does not consume the CLI adaptive-idle lease.
- Android `VideoTransport.connect` sets a 10-second read timeout;
  `VideoReceiver.connectAndReceive` treats timeout as reconnect. Suppressing all
  repeated frames without revising that contract would cause periodic reconnects.
- Host `selection/health.rs` already distinguishes idle content from outstanding
  unrendered work. Preserve that behavior rather than adding frame-count-based
  failure detection.

These are source observations. No new performance or power measurements,
physical suspend/lock, device mode changes or downloads were performed.

## Implementation direction

T913 defines a portable state/policy contract. Keep explicit host lock, display
power, machine availability, viewer presence and unchanged content distinct.
A quiet desktop can still be in use. Native adapters provide events; UI and
configuration consume independent states and user-configurable choices. Unknown
power state must remain explicit instead of pretending it means suspended.

T914 composes native events with the established capture gate, attachment and
retained resource lifecycle. Stop costly display work and release eligible
Android decoder/wake ownership during a display pause, retaining configuration.
Do not revoke independent camera/audio leases for host lock/display-off. Resume
only after old work really retires; revalidate the current authenticated peer,
restore fresh codec configuration and request a keyframe. Prevent rapid power
transitions from creating duplicate workers or reconnect storms. A broken cable
and host shutdown cannot be distinguished from transport loss alone. Lost local
registration IPC also needs exact broker route retirement or verified death of
the original native broker before quarantined ownership can be reused.

T915 separates content updates from connection liveness. Use existing damage and
cursor evidence to avoid unnecessary conversion/encoding/payload delivery, while
retaining first-frame, mode/config change, keyframe request and late-join recovery.
Audit FFmpeg input timestamp buffering and each supported decoder path before
removing compatibility repeats. Any replacement heartbeat must be cheap, bounded,
authenticated and unable to hide pending decoder/output failure. Retain a
configurable compatibility fallback where sparse operation is unvalidated.

Permanent tests must cover static/motion/cursor transitions, rapid pause/resume,
late callbacks, stale authority, reconnect, configuration changes and stuck
cleanup. Measure local CPU, encoded/payload bytes and first-changed-frame latency
with matched settings. Battery savings and live device recovery require a later
owner-approved physical window; existing deferred campaigns stay separate.

## Idle scheduling follow-up (T918)

Local inspection also found a 10 ms default timer in both Android connection
actors. The local endpoint factory currently supplies no native wake callback,
so progress relies on the periodic fallback. T918 tracks completion-driven
wakeups and nearest-deadline sleeps, with retained lost-wake fallback and matched
CPU/wakeup/latency measurements. This is separate from video-frame suppression;
no measured power saving is claimed.
