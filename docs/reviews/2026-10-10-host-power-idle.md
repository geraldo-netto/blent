# Host power following and unchanged frames

Owner decision (2026-10-10): host lock or display-off pauses display sharing only;
active webcam/audio continue. Host suspend/shutdown necessarily interrupts all
resources. Resume should restore enabled resources without restoring stale
connection authority. Portable policy is implemented under T913; native integration and unchanged-frame handling remain T914/T915 in TODO.md.

Owner decision (2026-10-12): retain D-Bus for Linux power/session observations
regardless of distribution. The adapter depends on the compatible
`org.freedesktop.login1` provider, rather than the distribution name or init
implementation. Absent/incompatible providers remain explicitly unsupported;
these private API fixtures do not establish native acceptance on every distro.

## Native implementation, verified locally before T914 integration

- `host/evdi/capture.c::on_dpms` records `dpms_on` and logs it. No production
  capture decision reads that flag; it also appears in diagnostic statistics.
- T1008 now supplies a read-only logind suspend/shutdown/resume and selected-session
  lock observer. It is not activated in the daemon yet; T1009/T1010 own composition.
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
registration IPC requires exact broker route retirement or verified death of
the original native broker before quarantined ownership can be reused. T1007 now
supplies retained Linux socket-bound process-exit proof, combined with actual
local native cleanup; unsupported observers retain conservative quarantine.
Linux logind/compositor observations and host display/auxiliary/Android lifecycle
integration are decomposed under T1008–T1011. T1013 supplies direct X11
display-power observation; T914 retains final acceptance.

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

## Portable policy and independent ownership (T913, 2026-10-12)

`common/src/host_activity` defines independent host observation, capability, policy
and resource authority contracts. The policy accepts strictly increasing revisions
from one positive source epoch. Rejected stale, foreign, duplicate or unsupported
observations leave its state unchanged. Supported-but-unknown machine power
conservatively pauses all resources; supported-but-unknown lock/display state
pauses display when its preference is enabled. Unsupported capabilities remain
explicitly false and preserve existing behavior. Transport loss is not a power
observation, and unchanged content remains a separate cadence input.

The persisted `[host_activity]` settings default to:

```toml
pause_on_lock = true
pause_on_display_off = true
pause_without_viewer = true
```

Disabling a setting preserves display demand for that condition, trading potential
idle resource savings for uninterrupted sharing. Machine suspension/unavailability
is mandatory for all resources regardless of these display preferences. Lock,
display-off and viewer loss leave auxiliary camera/audio demand enabled. Each
resource still requires its own exact current original authority; auxiliary demand
never grants a webcam/audio lease. Settings merge by edited leaf, preserving
concurrent changes to other preferences and media settings. Changes currently
require the existing explicit restart boundary; native application is T914.

One `Gate` per display, webcam, microphone and speaker reserves independently
retained startup/native completion before exposing a factory. Stop is irreversible
for that original request; rapid resume waits for actual completion. A stalled
resource cannot block another gate. Replacement rechecks current original
authenticated authority, and request exhaustion fails closed without wrapping.
Backend adapters must recheck the plan/authority before acquisition and retain
the real startup owner independently of cancellation or task results.

These are portable decisions and ownership rules, not running OS event integration
or a claim of reduced native CPU/power. T1008/T1013 supply Linux sources;
T1009–T1011 supply display/media lifecycle composition and fresh codec/keyframe
recovery, with final acceptance retained under T914. T915 supplies cadence
changes. Physical lock/suspend/tablet acceptance remains postponed.

T1014 adds a portable `Feed` and independently defined `ActivitySnapshot` contract.
Each snapshot carries the latest display/auxiliary host stop revision. A coalescing
watch subscriber can see a resumed desired state while still determining that its
original worker must stop. Work acquired after the stop revision is unaffected;
old work remains subject to the existing Gate's actual retirement and current
authority checks. A changed source epoch or future/invalid start/fence fails
closed. No snapshot grants native acquisition or substitutes for cleanup proof.

Rejected source observations leave the feed unchanged. Explicit whole-source loss
is terminal for that original feed and clears supported observations to unknown;
unsupported capabilities preserve existing behavior. Viewer presence and unchanged
content remain consumer inputs, separate from recorded host stop fences. Source
loss does not fabricate a new native revision or infer shutdown from transport.
Capture composition is T1015, USB registration composition T1016 and joined host
display acceptance T1009; auxiliary integration remains T1010. This decomposition
preserves the remaining work and does not establish running pause/resume support.
T1018 supplies the shared source/feed lifecycle and actual native ownership.
Owner decision (2026-10-12): host lock/display-off must keep the virtual monitor in
the desktop layout while capture/encoding pauses. T1015 separates retained monitor
ownership from stoppable capture/encoder workers, preserving ordinary attachment
loss and shutdown cleanup. Removing/recreating the output and moving windows on
host lock/display-off is not an acceptable substitute for a capture pause.

## Owned source following (T1018, 2026-10-12)

`SourceFactory`, `StartedSource`, `SourceLifetime` and `Publication` are independent
common contracts. `host/src/activity_follow::Following` reserves capacity before
exposing asynchronous creation. Publication distinguishes pending, observed,
failed and stopped; observed values include immutable capabilities and retained
host stop fences. All-false capabilities explicitly mean unsupported. Supported
source loss publishes conservatively unknown fields and is terminal for that feed.
The Linux factory retains D-Bus and the separately owned X11 helper adapters.

Source lifetimes register the exact creator/task/child completion certificates.
Both last-lease destruction and every registered certificate must complete before
the reservation can be reused. Native task leases survive actual task destruction
and join; helper leases survive process reap and cleanup. Dropped or timed-out
creation cannot release a late original native owner. Logical stop publishes its
terminal state immediately and wakes the controller; late publication cannot revive
it, and its actual retirement remains a separate observation.

The controller consumes at most the configured source queue capacity per turn.
Concurrent replenishment leaves events for a later turn rather than pretending
that the source overflowed. The adapter owns actual queue overflow enforcement.
Identical snapshots do not wake subscribers. Replayed/foreign/invalid events and
source errors close following conservatively; creator and observer panics cannot
publish an acquisition grant. Each resource still requires its own current
authority and actual retirement under the existing Gate.

Defaults are a 10-second total creation deadline and 50 ms observation interval;
valid intervals are 1 ms–1 second and creation deadlines positive–600 seconds.
Existing source defaults remain 2 seconds per native operation and 32 events.
Short intervals improve event response at a polling cost; larger intervals reduce
polling but delay following and need enough queue capacity for transitions. The
`BLENT_ACTIVITY_LIMIT` bounds concurrently active/retiring source owners, default 1,
range 1–4; exhausted capacity rejects before creation. These are typed/backend
controls; persisted user-facing capacity controls remain T898.

This controller and the Linux factory are validated library components. Daemon
capture/monitor and independent auxiliary composition remain T1015/T1016/T1010,
with Android and final acceptance under T1011/T1009/T914. Native private fixtures
do not establish physical lock/suspend, monitor-layout retention, distro-wide
acceptance or power savings. No such production integration claim is made.

Permanent normal-suite tests exercise all capability byte values, all eight
preference combinations, invalid TOML values, source/revision boundaries, concurrent
settings edits, original authority loss, counter exhaustion and 32 independent
resource cycles while actual retirement is held. Native counters and source
attestation are retained in [T913 evidence](artifacts/2026-10-12-t913/README.md).

## Linux observation adapters (T1008, 2026-10-12)

T1020 (2026-10-16) now retains the original capture helper's native completion
before awaiting its startup handshake. Cancellation or malformed startup cannot
permit another helper while original reap-dependent cleanup is held. Permanent
normal-suite cancellation/error regressions and joined CLI/SDK per-function
counters are retained in [startup ownership evidence](artifacts/2026-10-16-t1020/README.md).
Retained-monitor native pause remains T1021; source/capture and late stdout
publication composition remain T1015. This prerequisite is not activity-runtime
acceptance or a claim that a physical display was paused.

`common/src/host_activity::SourceSettings` defines independent capacity/deadline
contracts. `Observations` preserves each distinct full-state transition with
strictly increasing revisions in one immutable epoch. A full queue discards its
old snapshots, publishes unknown at the next revision and rejects further input;
counter exhaustion closes publication without wrapping. Invalid fields leave the
state unchanged. Stop discards outstanding observations and prevents late delivery.

`host/src/activity::LinuxSource` implements the independent Source interface.
One retained task owns logind power/lock observations and another owns supported
Wayland compositor display-power observations. Each original D-Bus service is
pinned by unique owner name; ownership replacement or malformed input makes only
that backend's fields unknown. A later service does not inherit the original
source. Fresh source owners use new epochs. Stop remains separate from actual
native task retirement; polling performs no native I/O.

Power uses logind's PreparingForSleep/PreparingForShutdown snapshots and explicit
PrepareForSleep/PrepareForShutdown signals, preserving overlapping preparation
states. Lock reads the selected session's LockedHint, never the Lock/Unlock request
signals or IdleHint. Session selection first uses GetSessionByPID; an explicit
local XDG_SESSION_ID fallback must resolve under the native session namespace and
match the current user's UID. Missing session selection reports lock unsupported.
These APIs report logind's observations, not physical screen/lock qualification.
See the [native logind contract](https://github.com/systemd/systemd/blob/main/man/org.freedesktop.login1.xml).

Wayland Cinnamon/Mutter selection uses the original compositor's PowerSaveMode:
0 is on; 1–3 are off; -1 is unknown/unsupported. Other values reject. On X11 this
property may be stale after direct XRandR changes, so automatic selection leaves
that compositor capability unsupported on X11; T1013 supplies direct DPMS
observation there.
Other desktop interfaces also remain unsupported, with no hardware support claim.
The [Muffin property contract](https://github.com/linuxmint/muffin/blob/master/src/org.cinnamon.Muffin.DisplayConfig.xml)
defines these values and the XRandR limitation; a read-only current native query
returned 0. No power-setting method is called.

Native receive-position fences reject queued/replayed state older than each
initial property snapshot. Invalidated lock/display fields first become unknown,
then a bounded query to the same original owner may restore them. A second owner
check prevents that reply from restoring authority after name replacement. Native
payloads are limited to 8192 bytes before typed decoding; changed/invalidated
property collections have at most 64 entries. Stock zbus framing/allocation remains
inside the external dependency; this is a bound on Blent's decoding/publication.

Source defaults are a 32-event queue (range 1–256) and a 2-second deadline (positive,
at most 60 seconds). The deadline applies to each bus connection/adapter startup
phase and to the entire invalidation refresh plus owner check. Larger queues retain
more distinct transitions at a memory cost; longer deadlines tolerate slow native
services while delaying startup or recovery. Source settings are typed parameters;
user-facing connection capacity controls remain T898. Runtime composition and
physical suspend/lock/display acceptance remain T1009–T1011/T914. No CPU or
power improvement is claimed from these observation fixtures.

## Direct X11 observations (T1013, 2026-10-12)

An X11 session uses an independently owned, read-only DPMS helper. It queries
extension support, capability and current power through the installed Xlib/Xext
ABI. Enabled power level 0 is on; 1–3 are off. Disabled DPMS means on, with its
undefined level ignored. Invalid native booleans/status/power levels reject;
missing libraries, display access or DPMS capability report unsupported. This
does not replace D-Bus power/lock or Wayland compositor observations.

The existing Linux executable dispatches the private helper before daemon startup;
there is no new package or native link dependency. Only that isolated process
makes blocking Xlib calls, including connection and close. It emits bounded
typed JSON snapshots over its original stdout pipe. Heartbeats repeat at the
smaller of 250 ms or one quarter of the configured observation deadline (minimum
1 ns); sample timeout/lost output makes only display power unknown. No replacement
process inherits original authority. Blent bounds a frame to 256 bytes before
decoding, checks the exact schema and rejects capability withdrawal.

Logical stop wakes the async observer immediately. The source's retirement proof
also retains the actual creator and original child, independently of task results.
A creator that misses its startup deadline remains owned until it returns; any
late child is then retired before ownership is released. A blocked Xlib connection
cannot terminate or stall the daemon, and cancellation alone cannot free capacity.

Permanent normal-suite tests exercise native ABI transitions/rejection, disabled
and unsupported DPMS, a private Xvfb connection, the real executable dispatcher,
a real Xlib connection blocked by a private TCP server, bounded pipe framing,
timeout/loss, independent other fields, and a held creator with late cleanup.
The heartbeat regression fails with the fixed 250 ms interval and passes when
the configured deadline controls it. These establish the adapter and ownership
boundaries; supported physical monitor transitions and joined host/tablet pause
and resume remain T1009–T1011/T914. No desktop power-setting methods, forced host
lock/suspend, tablet interruption or distro-wide qualification was performed.

## Native retained-monitor pause boundary (T1021, 2026-10-16)

The Linux capture helper accepts an optional private `--capture-activity-fd`
sequenced-packet endpoint and starts paused when it is present. Its independent
nine-byte request/acknowledgment schema is shared with the portable contract
added under T1023: a nonzero, consecutive big-endian u64 sequence followed by a
strict boolean. The helper echoes the exact record only after the transition
completes. Malformed, replayed, out-of-order, truncated and descriptor-bearing
commands fail the helper; acknowledgments never imply unrelated process cleanup.

Pause retains the connected monitor, registered native framebuffer, mode and
capture process. It stops update requests, pixel grabs, NV12 conversion and frame
publication while continuing native/control event service. It invalidates the
original FIFO frame generation exactly once. A writer that exceeds the bounded
retirement wait retains its original immutable lease and every allocation; the
pause acknowledgment stays pending until that same writer releases. Subsequent
iterations observe completion rather than invalidating again or replacing owners.

Resume invalidates damage and obtains a fresh pixel snapshot before another
update request, preserving the native flip acknowledgment ordering. An unchanged
snapshot still seeds a full frame for the new encoder. FIFO reader transitions
and shared-raw control continue to work. Shared-raw reader references remain owned
by their original encoder process: this native producer acknowledgment does not
replace the separate encoder retirement certificate required by T1015.

Permanent fake-device tests cover held leases, queued callbacks, retained monitor
and buffer identity, malformed records, acknowledgment backpressure and received
descriptor cleanup. A real Rust exec/socket handoff drives sixteen transitions
through the production C event loop and asserts fresh resume ordering before each
acknowledgment. Sanitizers cover native accesses. These are component acceptance
tests; application activity composition remains T1015/T1016/T1009, and physical
host/tablet acceptance remains T914. No lock, suspend or display setting was
changed, and no runtime power/CPU improvement is claimed from these fixtures.

T1025 moves the fallback deadline into the capture owner's pacing state. A
successful resume snapshot advances that same deadline, so the first resumed
tick does not perform a second fallback grab before the configured request is
due. The 250 ms lost-update watchdog and subsequent fallback recovery retain
their existing deadlines; permanent mock-clock tests cover both paths.

T1022 bounds helper announcements to 1024 wire bytes per record, including any
LF/CRLF terminator. Fixed-size read buffering and bounded accumulation reject
oversized terminated or unterminated lines before payload growth. Partial bytes
remain owned across canceled reads. Standard UTF-8, blank-line, CRLF and final
unterminated-line behavior is preserved, including a lone CR at EOF. Framing or
I/O failure poisons that reader. EOF or stdout task failure wakes the capture
owner and remains failed on repeated waits until fresh successful startup. The
original native child and its independent cleanup certificate remain retained;
stdout task completion is never native process retirement.

T1024 gives each original stdout drain a private publication authority. Revoking
it precedes cancellation and synchronizes each watch update; callbacks check
authority inside the same write transaction. Old mode, stream and FIFO notices
cannot cross revocation into current state. The original stdout task completion
and FIFO lease survive asynchronous abort, and helper replacement checks that
completion independently of native child cleanup. Reentry with a live helper or
owned stdout task rejects before FIFO changes or another spawn. Termination waits
within the existing configured reap budget; expiry retains the original pending
certificate. Fresh startup clears the old stdout failure latch only on success.
Whole-session dependent retirement and activity propagation remain T1026/T1027;
display policy composition remains T1015.
