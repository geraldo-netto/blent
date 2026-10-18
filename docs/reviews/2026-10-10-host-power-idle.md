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
- T1008 supplies a read-only logind suspend/shutdown/resume and selected-session
  lock observer. T1027 now starts one original source through
  `linux_main.rs::run_daemon_with`/`daemon_activity::start` and passes read-only
  publications to primary and extra sessions. Selection consumes the original
  publication under T1037; T1015 now guards native display startup and running
  work with the same machine/viewer view and original attachment lease. Healthy
  helpers retain their monitor during activity pause while capture/encoding
  retire. Auxiliary integration remains T1010; USB registration renewal remains
  T1016, and joined host/Android acceptance remains T1009/T1011/T914.
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
- Android `VideoTransport.connect` retains a 10-second read timeout;
  `VideoReceiver.connectAndReceive` treats expiry as reconnect. T1038 now supplies
  bounded independent liveness records after the original socket's explicit
  protocol-1 acknowledgement; those records keep negotiated no-frame connections
  alive without decoder/frame/render callbacks. Legacy viewers retain their
  existing framing and timeout behavior. T915 still owns safe frame suppression
  and its configurable compatibility fallback.
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
Whole-session dependent retirement is implemented under T1026/T1028; auxiliary
registration and activity propagation remain T1029/T1027;
display policy composition remains T1015.

T1026 adds an independent native-retirement recorder to the portable capture
contract. Recorder clones retain creation authority; each original task/thread
and dependent native resource separately registers its exact completion before
exposure or another await. The session installs retained ownership before calling
the capture factory, including factories that panic after native exposure.
Supervision retains admission until all parent tasks, original creators and
registered native completions finish. Parent success, failure, panic, cancellation
or supervisor completion cannot free that admission. Completed registration
history is pruned across repeated restarts; failures stay visible during cleanup.

Portable acceptance covers held original native admissions, real owned blocking
creators that produce late resources, independent sibling progress, factory panic
and runtime route retirement after supervisor exit. These fixtures verify the
reporting/supervision contract. Linux helper/encoder registration is implemented under T1028; auxiliary
command/idle registration remains T1029 and activity source wiring T1027;
the portable contract alone does not establish native runtime support.

T1030 keeps owned-task admission through dependent lease destruction after join.
Cleanup uses the existing isolated retirement worker under the original budget,
so a blocked destructor retains its own certificate without blocking unrelated
retirements. Joined work results remain distinct from resource completion.
Helper wait retains the original stdout certificate when post-join cleanup is
pending. X11 startup retains its original creator certificate in startup state;
both initial and repeated joins wait for that proof even after a canceled wait.
The unchanged unsupported-program startup regression and added held-cleanup
regressions pass. No elapsed deadline or destructor entry certifies completion.


T1028 binds the original session recorder before Linux capture construction can
start native work. Helper startup records its native child before handshake or
fallible stdout setup, and records the drain task before exposure. CLI, optional
GPU and isolated native encoders record their actual child and packet-reader task
proofs before another fallible step or await. The isolated worker adapter also
records children whose subsequent reader admission fails. Shared FIFO/raw leases
remain owned by those same original native resources. Whole-session stop/drop
cannot release admission while any registered cleanup remains held.

Permanent real-child and task fixtures cover successful helper/CLI startup,
stdout and packet-reader post-join leases, canceled/failed helper handshake and
post-spawn encoder task admission failure. Their enclosing session can finish
while exact native cleanup remains pending; release is certified only after
actual original completion. This closes core native registration, with auxiliary
probe/idle registration T1029 and activity composition T1027/T1015 still open.


T1031 adds original completion observation to the shared bounded synchronous and
asynchronous command adapters. Both existing and observed calls execute through
one implementation. Collector completion now remains available in production;
its actual thread join is independent of delivered output or cancellation.
Collector proofs register before child creation; child proofs register after
successful construction and before awaiting process/output completion. Sync
observation precedes Windows setup; async Windows assignment is internal to
construction and its failure/reporting boundary remains unresolved under T737. A synchronous
observer panic retires the already exposed original child before unwinding.
Command deadlines, output bounds and sensitive stdin handling remain unchanged.
Caller-owned blocking creators must retain their own reporting lifetime until
late command creation is impossible; capture/selection wiring remains T1032.
Native Windows validation of these observations remains explicitly under T737.


T1032 propagates one immutable original reporting scope through Linux selection,
async-depth queries, throughput/format/quality probes and cache creation/writes.
Probe children now use retained native ownership and publish original proofs
before another fallible step or await. Bounded commands forward both child and
collector proofs through the same scope. Blocking cache creators retain their
actual owned-task join certificate and reporting lifetime independently of their
parent future or returned value. A canceled creator can still report a later
child, whose original dependent cleanup continues to fence session replacement.
Standalone probes use an explicit unbound scope; no runtime or OS authority is
invented by that value. Public capture contracts remain independent of this
implementation adapter.

Retained regressions cover canceled creators after alias release, actual held
child cleanup, a later child exposed after cancellation, whole-session stop during
a live held probe, command forwarding and cache creation/persistence reporting.
All pass with individual native counters. The global calibration semaphore is a
separate remaining owner: its parent currently releases the permit before native
proofs settle. T1033 records that contradiction and owns its independent fix;
idle task retirement remains T1029. Activity integration is still T1027/T1015.


T1033 retains the global calibration permit through original native retirement.
Each calibration gets an independent observation group that forwards all original
proofs to its enclosing session and retains late creator aliases. Dropping the
parent revokes its group alias first. If that group is already complete, the
same permit and admission release immediately; otherwise their original owner
transfers into the bounded retirement observer. Neither a canceled parent nor a
joined creator permits replacement while a later child remains live. The
original three-second admission deadline covers both semaphore acquisition and
pending retirement-budget handoff. Closed admission rejects without native work.
Cached healthy profiles continue to bypass this workflow. Real preparation,
held child/creator, late native creation and sibling regressions pass; source
and native function evidence is retained with T1033. Idle ownership remains
T1029 and activity composition T1027/T1015.


T1029 moves idle controllers into original owned-task admission and reports
completion before exposing a handle. The same task retains its dependent lease
through actual join and isolated post-join cleanup. Work-result readiness remains
a distinct query. Encoder readers retain the exact original idle completion and
wait for it before a successful rebuild/rotation boundary, including when the
reader result has already been consumed. The existing encoder-stop budget covers
that wait; cancellation and expiry retain the original proof and forbid successful
replacement. Capacity exhaustion rejects idle startup and preserves compatible
cadence. Existing lease revocation and successor-token behavior remain unchanged.
Held task, post-join dependency, deadline/retry, production startup and capacity
regressions pass in normal suites with native function counters. File effects
inside idle drop/run remain synchronous and are independently tracked as T1034;
this ownership result does not establish bounded control-worker responsiveness.


T1034 moves idle lease creation, cadence file effects and final dependency cleanup
off Tokio control workers. One owned native thread runs the existing portable
policy against its Linux file adapter; bounded history and coalesced activity/
stop notifications remain unchanged. Revocation publishes an atomic stop and wake
without taking the file mutex. Original native join and lease destruction remain
separate, retained completion requirements. This uses dedicated native ownership
rather than a runtime-owned blocking task, so a stuck file effect does not force
Tokio runtime destruction to join it. Shared async/native admission retains the
existing configured task limit through one independent capacity boundary.

Async encoder startup awaits native lease readiness after reporting the original
creator. Queued lease views drop before native retirement is exposed; a retained
lease registry also covers creation after parent cancellation. Actual thread
join/TLS teardown and dependent destruction use the established isolated
retirement helpers. Panics, failed/closed startup and already-revoked requests
fail visibly without granting replacement. The held-file regressions originally
blocked caller drop/runtime for about 400 ms and now meet their <100 ms control
bound while original cleanup stays pending. Existing cadence, token and timing
assertions remain, with explicit native completion barriers replacing assumptions
about Tokio task scheduling. Activity source/policy composition remains T1027/T1015.


T1027 composes one activity follower at the Linux daemon boundary, using saved
host-activity preferences and the existing native source factory. Primary and
extra ADB/USB construction receive clones of the same original read-only watch;
portable prepared sessions forward that watch into the independent capture
context. Linux capture retains the view without creating another feed/source.
Other compositions explicitly default to no supported activity source. Original
capability reports, loss state and coalesced stop fences remain intact.

Daemon shutdown requests source stop and waits for original native retirement
within the existing shared shutdown budget. Expiry/cancellation drops the same
retained follower; it cannot manufacture completion or release native admission.
The OS signal adapter is separate from daemon orchestration, allowing normal
startup/shutdown validation with an injected source in private PID/runtime/config
namespaces. Fixtures disable input, updates and native USB startup, expose no
attached tablet, and never change a live display. Actual display pause/resume
policy remains T1015; auxiliary machine-availability policy remains T1010.


T1035 adds an optional Linux helper activity adapter. It attaches only the
reserved inherited endpoint before spawn and binds the original controller socket
lease to the same native child immediately after construction. Failed/canceled
handshakes retain that lease through actual child cleanup. Stop/terminate revoke
issuing access while native resource retention remains independent. Requests use
the existing strict acknowledged transport, including its original pending
transition across caller cancellation and terminal foreign-ack rejection. Each
fresh helper gets a fresh channel/sequence, only after original child/stdout
retirement permits restart. Shared raw and activity endpoints remain independent.

This adapter remains explicitly opt-in. Controlled helpers start capture inactive,
so enabling it in daemon composition before policy integration would stall normal
streaming. The current daemon therefore retains its existing uncontrolled helper
behavior; T1015 owns enabling it together with source-driven pause/resume. Seven
CLI helper regressions and an additional SDK shared-raw coexistence regression
use harmless native subprocess fixtures. Removing original socket retention
reproduces both held-startup failures; the unchanged assertions pass after restore.
No physical monitor/power acceptance is implied by these process/socket checks.


T1036 reserves an independent original reporting group before encoder acquisition,
including asynchronous capability preflight and optional GPU startup. The original
observation is stored before the creator is exposed; cancellation drops only the
creator view. Actual task/native proofs and late creator aliases still forbid a
new generation. Successful acquisition retains one active reporting view through
child, reader and idle creation. Closing revokes that view independently of the
read-only completion observation. Every original proof also reaches the enclosing
session through the existing reporting boundary.

Encoder construction now owns its private generation state and GPU adapter setup.
Session startup adopts already-owned children into the same scope; SDK worker
creation errors close the unexposed generation without inventing native completion.
Both encoder replacement and FIFO recovery/rotation check original generation
retirement. Helper/monitor ownership remains outside that encoder group: the
same actual harmless helper process survives repeated closed encoder generations.
Policy-specific waiting with a retained physical monitor remains T1015.

Six initial retirement assertions fail before group observation and pass unchanged
afterward. Retained cases cover canceled production preflight, creator alias versus
actual join, joined reader dependency, later child surviving creator join, actual
idle file cleanup and repeated generation boundaries. Additional native spawn
failure and bounded invalid SDK job regressions preserve rejection/retry behavior.
Fresh CLI/SDK counters and normal suites pass. T1038 records the independent
Android 10-second read-timeout contradiction discovered while reviewing no-frame
activity integration; transport liveness must land before host pause is enabled.


T1039 enforces the accepted explicit-acknowledgement boundary before new liveness
packets can be emitted. An acknowledgement follows authentication on the original
video TCP socket, retaining the existing optional legacy-token prelude. Only the
exact bounded eight-byte version-1 record publishes the connection-owned grant;
its first byte starts the existing three-second completion budget. A read-only
view becomes unsupported when its original publisher closes. Other sockets never
inherit that watch or its grant. At the T1039 checkpoint, host advertisement remained absent pending T1038's
emission path; legacy traffic kept its existing framing.

Android accepts offers only from its current protected connected greeting. That
configuration travels separately from codec format in an immutable display-worker
request and retires prior display work before replacement. The video adapter
writes authentication first, then the acknowledged feature record, checking
original socket/generation ownership around the write. Blocking writes retain
only their original socket and stop can close it without waiting for the receiver
monitor. The parser accepts version-1 liveness only when that connection enabled
it; transport progress produces no decoder input, first-frame connection event or
render acknowledgement. Unrelated audio/camera ownership is unchanged.

Permanent red/green regressions cover strict bounded records, partial deadlines,
legacy silence, failed authentication, source closure/replacement, malformed offers,
immutable worker boundaries, stale queued UI callbacks, blocked acknowledgement
cancellation and frame isolation. An additional draft regression reproduced an
optional observer throwing before control revocation/socket close; protected
notification now preserves cleanup and the same oracle passes. Native function
coverage and full normal suites pass. T1041 owns exposing original negotiated
viewer capability to capture, and T1040 records bounded early invalid-packet
read/allocation work; neither is implied complete by negotiation readiness.


T1038 enables bounded frame-independent heartbeats only after the original video
socket acknowledges protocol 1. The protected media control greeting advertises
that offer; input-only compositions omit it. A six-byte record renews transport
progress without codec/frame/render callbacks. The two-second default is
configurable from 100 to 5000 ms; invalid values reject before capture subscription
or IDR demand. Successful configuration/frame/liveness writes renew the deadline;
obsolete producer traffic cannot starve it. Legacy sockets wait without a heartbeat
timer, late acknowledgement wakes the same playback, and closed grants deny writes.
Authentication, attachment fencing, initial five-second CSD wait and existing
one-second write/batch budgets remain in force. Fresh resume still requires the
current generation's configuration and IDR. These records retain a quiet TCP
connection; registration policy, retained monitor and Android decoder/wake activity
policy remain their separately tracked integration work.

The retained T1038 suite passes 787 host-library, 343 shared-contract and 1,996
Android tests. Twelve scoped Rust production functions measure >=95.65% native
line coverage; unchanged Android production methods retain T1039's native
counters. The source audit covers 36 changed functions at complexity <=9; native
SonarQube remains unavailable under T1019.


T1041 projects original video-socket grants through independent read-only capture
contracts. Receiver ownership and weak registry membership share one lifecycle;
actual subscription and membership retire together. Readiness requires current
viewers and acknowledgements from every member. Legacy/unmanaged/closed sources
deny peer-dependent pause eligibility. Synchronously armed structural/source
notices wake on ACK, replacement and loss; observation retains neither mutable
grants nor receiver/native authority. Old notices cannot retain membership or
retarget replacement grants. Empty/already-closed views wait without polling.
The existing production sixteen-client admission bounds concurrent TCP sources.
Permanent actual-authentication, mixed-viewer, 1,000-cycle late-replacement and
membership-retention regressions pass. Fresh counters cover all 35 scoped
production functions at >=94.74%; 794 host-library, 343 shared-contract, two native
FIFO and seven architecture/portability checks pass. The source audit covers
50 scoped functions with maximum complexity 9; native SonarQube remains T1019.


T1043 centralizes desired-work interpretation in the independent
`WorkObservation` contract and host `activity_view::View` adapter. Each immutable
observation binds its target and original stamp, retains coalesced stop fences,
and rejects invalid/future/foreign state, supported-source loss/closure and target
mismatch. Explicit unsupported target capabilities preserve their fallback on
loss; no-viewer demand remains independent. Lock/display pauses affect display;
suspension affects both display and auxiliary work. The original watch adapter
arms before inspection and wakes on changes/closure before first poll, without
copying mutable source state or adding idle polling. It grants no native or
attachment authority. Five initial independent regressions fail before strict
interpretation and pass unchanged. All eleven production functions have 100%
native line coverage; 797 host-library, 350 shared-contract and seven architecture
checks pass. Source scores cover 25 functions at complexity <=9; native SonarQube
remains T1019. Runtime consumers remain their separate integration items.


T1037 gates automatic selection with the same original activity publication and
shared work observation as capture will consume. Each trial captures the original
attachment lease; synchronous profile-change notices prevent same-identity route
replacement from disappearing behind unchanged settings. Current activity/profile
and local settings/mode/stop inputs are checked around every workflow poll and
ready result. Coalesced stop fences invalidate completed trials even if the
selection key is unchanged. Supported-source loss/closure and pending/failed/stopped
publications deny work; explicitly unsupported capabilities retain their fallback.

A reporting group is reserved before cache/environment preparation, not only
calibration admission. Canceled parent return cannot admit another trial until
all original native creators/proofs finish. Stop leaves those proofs retained by
the enclosing session rather than pretending cleanup succeeded. Permanent tests
hold the actual cache fingerprint FIFO before admission and observe the original
late version subprocess through cleanup; native resources and no physical capture
are isolated in harmless fixtures. Eight new regressions join the normal suite.
Per-session viewer preference composition remains T1045, with native retained-monitor
pause/resume and final display acceptance still T1015.

Final T1037 suite passes 506 tests with four existing ignores. Fourteen scoped
production functions measure 100% native executable-line coverage; the source
audit covers 36 functions at maximum complexity 8. Seven architecture checks and
the separate T1044 production compilation regression pass. Native SonarQube remains
T1019, and these fixtures establish no physical monitor acceptance.


T1045 composes machine observations with original per-session viewer membership.
Saved `pause_without_viewer` reaches primary and extra ADB/USB prepared sessions
and their independent capture context. One read-only display-work view is shared
by selection and the capture adapter; selector now honors the saved local viewer
preference. Native capture consumption remains T1015.

Viewer continuity belongs to the original receiver registry. The registry replaces
an opaque immutable continuity identity only after membership becomes empty;
coalesced last-viewer replacement therefore cannot revive old work. Additional
viewers and departures that leave another viewer present do not invalidate it.
Retained observations/notices own no receiver, grant or native resource. Presence
is independent of liveness acknowledgement: legacy viewers count as viewers,
and an ACK without actual receiver membership does not create demand. Explicitly
unsupported presence observation preserves its fallback; disabling the preference
keeps work eligible without viewers. Machine stop fences/loss still deny work.
The adapter adds no idle polling or copied mutable grant state. This local demand
composition does not establish retained-monitor runtime or physical acceptance.

T1045 final native suites pass 808 library and 508 daemon tests, with three/four
existing ignores. Thirty scoped production functions measure at least 83.33%
executable-line coverage individually; 28 measure 100%. The source audit covers
79 changed functions at maximum complexity 8. Seven architecture checks pass;
native SonarQube remains T1019.


T1047 fixes encoder retirement retry after cancellation between original reader
join and idle-controller native cleanup. Previously a retry polled the consumed
`Task` handle and panicked. A read-only joined-result observer checks the original
handle state, without copying completion state or granting native retirement.
The permanent held-idle regression proves the cancellation point and reproduces
the original panic before the fix; unchanged assertions now permit safe repeated
retirement only after exact cleanup. Existing error/panic semantics remain tested.
This prerequisite supports T1046's interruptible retained-monitor transaction.

T1047 native capture suite passes 160 tests with two existing ignores. Both scoped
production functions measure 100% executable-line coverage; the source audit
covers three functions at maximum complexity 7. Native SonarQube remains T1019.


T1046 adds one retained-monitor quiescence transaction: stop the original encoder
child/reader alongside the original helper pause ACK, await exact native generation
creators/proofs, then clear headers, request fresh IDR and rotate the FIFO. The
helper/card/stdout/control ownership stays intact. Cancellation retains the
original request and quarantine. Original reader join keeps making progress after
normal stop grace; its timeout cannot substitute for cleanup or abandon the
still-owned result. Foreign ACK and uncontrolled helper operation fail explicitly.
Seven permanent native fixture regressions include sixteen same-helper cycles,
held cleanup, canceled ACK/native startup, malformed ACK and post-grace blocking
reader completion. These are process/socket ownership fixtures, not physical
monitor acceptance. Daemon enabling and guarded resume remain T1015.

T1046 final normal CLI suite passes 516 tests with four existing ignores; installed
SDK passes 81 capture and nine encoder-worker tests. All five production functions
measure 100% native executable-line coverage in each configuration. Nineteen
changed functions measure source complexity <=7; native SonarQube remains T1019.
Seven architecture checks and normal production compilation pass.


## Guarded display runtime (T1015, 2026-10-16)

Primary and extra Linux ADB/USB capture managers now consume the same original
machine/viewer work view as selection, plus the original attachment profile.
One current-work guard checks before and after each workflow poll/result;
selection shares that check instead of retaining a second implementation.
Original source/profile notices are armed before inspection. Pending, failed,
stopped or lost supported sources deny work; explicitly unsupported capabilities
retain their fallback. Saved no-viewer policy remains independent of transport
ACKs. Coalesced stop/resume and same-identity attachment changes still retire
the original work before a new attempt, even when current settings match.

Managed helpers start inactive. A current guarded attempt awaits exact original
encoder creators, reader and native completion proofs before preparing and
resuming capture. Activity pause uses T1046's original helper acknowledgement
and native quiescence, clears headers, requests fresh IDR and rotates the FIFO
while retaining the healthy helper/card/control/stdout owner. Desired resume
cannot bypass pending retirement. Ordinary explicit display detach and shutdown
continue removing the output. Invalid resume acknowledgement retires the helper
and uses bounded recovery backoff; retained output is not promised after native
failure. Existing settings, input and independent media owners remain separate.

A permanent regression also reproduced stale header ownership on explicit
display-mode restart: identical header bytes were deduplicated across retired
generations. Header ownership is now cleared after exact old generation
retirement and before replacement, including ordinary mode restart. Fourteen
new normal-suite regressions exercise actual synthetic helper processes, FIFO
pixels and CLI/in-process encoders: pending startup, running/coalesced pause,
profile replacement, local viewer policy, source loss/closure/failure, unsupported
fallback, explicit mode detach, interrupted handshake, malformed resume ACK,
stale ready result and whole-manager shutdown/drop with held original native
startup. Retained original completion stays pending after the parent returns
and completes only when the actual held worker exits.

Fixtures use private sockets/processes and synthetic frames, with desktop
environment and executable paths isolated from physical capture. They establish
no physical monitor/tablet acceptance or power savings. Negotiated video
liveness remains T1038; legacy framing/read timeouts remain unchanged. USB
registration integration is T1016, joined host acceptance T1009, Android resource
handling T1011, independent auxiliary handling T1010 and physical acceptance
T914. Native SonarQube measurement remains T1019. No downloads, installation,
live lock/suspend or push occurred.

Final T1015 native suites pass 808 host-library and 530 CLI daemon tests with
three/four existing ignores, plus 95 SDK capture and nine encoder-worker tests
without ignores. Every scoped production function passes individual executable-
line coverage: 32 CLI and 30 SDK functions, minimum 80% in each configuration.
The source audit covers 65 changed functions with maximum complexity 7; seven
architecture checks and production compilation pass. The broad SDK private-
bus/namespace fixture prerequisites remain explicitly blocked under T835.
[Retained red/green logs, native counters and source/native-object attestations](artifacts/2026-10-16-t1015/README.md)
record the exact validation scope.


T1040 moves video type and type-specific size admission ahead of payload reads
and reusable backing growth. Generic invalid sizes consume only the four-byte
length; unknown types, short frames, unnegotiated liveness and malformed liveness
lengths consume only length plus type. Invalid headers now return false from
`read`; valid packets retain full-payload EOF behavior and synchronous borrowed
byte ownership. Version rejection remains after the valid two-byte liveness body
is read. Configuration/frame offsets, sequence interpretation, bounded growth
headroom and following-packet framing stay covered by the retained oracles.

The permanent T1040 tests exhaust unknown type bytes, cover generic and
type-specific length boundaries, and spy on reads and backing identity. The
[retained evidence](artifacts/2026-10-18-t1040/README.md) records failing-before
and passing-after results, the normal Android suite and native per-method
counters. This is an invalid-input efficiency correction, not a battery,
physical-device or unrestricted memory-safety claim.


T1049 adds reversible suspension to the portable renewable display registration
owner. Pause synchronously revokes only its original attachment/socket authority
and requests broker close while retaining configuration and desired presence.
The owner continues polling original retirement while paused and issues no new
registration. Resume requests fresh credentials only after actual client retirement
and the original retry deadline; it cannot revive terminal cancellation or replace
a newer attachment. Five permanent tests include 1,024 exhaustive five-transition
traces over pause, resume, client retirement and terminal close.

The [T1049 evidence](artifacts/2026-10-18-t1049/README.md) records native counters
for all 13 registration functions (minimum 84.62%), 552 passing library tests with
three existing ignores, and source complexity <=6. This supplies only registration
suspension. Independent native capture/selector fencing remains T1050, production
activity composition remains T1051, and T1016 retains joined automated acceptance.
A resume request is not native cleanup evidence or physical USB acceptance.
