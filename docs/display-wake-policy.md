# Android display wake policy

Status: independent desired policy (T1059), original-request admission (T1062),
handle ownership (T1063) and foreground native service ownership (T1060) are
implemented. Activity observation/window integration remains T1061; joined
acceptance remains T1058/T1011 and physical wake/power evidence remains T914.
Current production Activity requests still use Waiting. Native handling of the
other states is covered separately; it does not establish a measured power saving.

`StreamingPower` is an immutable, platform-independent policy input containing
battery-saver preference, authenticated media activity, observed transport and
display wake state. `StreamTransport` reports USB, network or unknown; charging
and a loopback address cannot infer the route. Its projection has no Context,
Intent, Service, Window, native lock, callback, queue or allocator dependency.

| Display state | Display wake demand |
| --- | --- |
| Waiting | Preserve normal/battery policy while a connection is pending. |
| Unsupported | Preserve the explicit fallback for hosts without observation support. |
| Active | Apply normal/battery policy to eligible display work. |
| Paused | Release display CPU, Wi-Fi and screen wake demand. |
| Unavailable | Release display wake demand after supported source loss. |
| Background | Release this Activity's display wake demand. |

For Waiting, Unsupported and Active, normal mode requests CPU and Wi-Fi wake
locks. Battery saver requests no CPU wake lock; it requests Wi-Fi only while media
is active on network/unknown transport. Wi-Fi withdrawal is immediate on USB and
uses the existing five-second idle grace on network/unknown transport. These
states retain screen-on demand while foreground. Paused, Unavailable and Background
request no locks or screen-on demand and bypass Wi-Fi idle grace. These are display
policy values; they do not revoke independent camera/audio consent or resources.

The framework adapter carries the same immutable fields in a private Intent.
An absent display-state field represents the explicit Waiting fallback. A present
unknown, null or wrongly typed state becomes Unavailable, including empty strings,
case changes and future values. Known values are fixed ASCII strings of at most
11 characters. Unknown transport remains unknown. The adapter consumes an existing
caller-owned Intent and returns one bounded value record; it retains no input
reference or native capability. The caller owns and bounds the containing Intent,
its extras and retained output copies. Synchronous conversion finishes without
creating cleanup obligations.

Eligibility is separate from ownership. Native adapters must validate the actual
original current foreground owner when admitting queued requests, applying locks
and running delayed release. Background/source-loss values cannot authorize a
later reacquisition. Actual wake handles remain owned by their service/window
adapters through partial failure, owner loss and cleanup; a hint or lock release
is not proof of decoder retirement and cannot authorize native decoder reuse.
Conditional eventual decoder cleanup still requires original worker/native
settlement and OS/scheduler progress, with unknown obligations retained and funded.

Permanent tests enumerate all 72 combinations of display state, battery policy,
media activity and transport against an independent table. Separate framework
tests round-trip those values on SDK 27 and 34, retain missing-field fallback,
and reject malformed-state demand, including a wrong native extra type and a
4,096-character invalid value. These tests establish value/adapter behavior,
not native lock timing, physical screen state or battery savings.

## Original request admission

T1062 supplies independent foreground-producer and native-receiver interfaces plus
a bounded request implementation. T1060 binds it to the foreground service.
One opaque owner identifies each original foreground claim; a request carries that
owner and an immutable desired policy, or null policy for withdrawal. Identity
does not wrap or derive from wire/process counters. Retaining an old view keeps
its identity distinct; it cannot compare equal to a new owner or current snapshot.

The channel retains one latest request, one dirty bit and at most one original
queued notice. Updates replace latest policy without issuing another notice while
that original notice is outstanding. Replacement immediately withdraws old producer
authority but retains the original queued admission. Its exact receipt can issue
one fresh notice for the replacement owner; it cannot return replacement policy
as an old owner's request. Foreign, duplicate and stale notices cannot consume a
new admission. Closure is terminal for its owner. A current original receipt after
closure yields withdrawal, and old producers cannot publish or close a replacement.
Applying a returned snapshot still requires exact current-view relevance.

Initialization has no owner/notice. Open installs a fresh immutable request before
admission. Publish/close require the exact live owner; closure installs withdrawal
before denying future writes. Receipt clears only the exact original notice, takes
latest state only for that same owner, then admits a still-dirty successor once.
The dirty bit always implies a latest request. These transitions preserve one
admitted notice and at most two retained immutable requests/owner identities within
the channel. Outputs are borrowed immutable views; callers bound additional retained
views and their containing native handoff lifetimes. No Context, callback, coroutine,
queue implementation or native handle is retained by this pure contract.

Unknown native delivery keeps the original notice and bounded capacity retained;
there is no timer-based settlement, counter reset or inferred native proof.
Conditional eventual notice cleanup requires original receipt and forwarding of
any returned successor. The backend must account for submission rejection, partial
failure and cancellation using actual original handoff evidence. Receipt only
settles this logical notice slot. Native service/lock cutoff, quiescence, settlement,
cleanup sponsorship and reuse remain separate T1060 obligations. In particular,
`current` is a snapshot relevance check, not an asynchronous native lease: native
adapters must serialize current-authority validation and effects within their
actual owner and keep original resource obligations through uncertain cleanup.

Permanent tests cover foreign/closed/replaced owners, retained snapshots, duplicate
notices and 1,000 coalesced updates. An independent finite model exhausts all 4,096
four-step histories over two producer slots, replacement, publish, close, receive
and duplicate receipt. Each trace allocates at most five local original owner names;
these are bounded test names, not production epochs. A deliberate foreign-owner
control is rejected by the permanent original-notice regression. The model covers
linearized channel operations, including forbidden stale publication/admission;
it does not establish native delivery fairness or unrestricted hardware safety.

## Original wake-handle ownership

T1063 supplies the independent WakeHandle contract and WakeLease owner; T1060
implements its Android backend. A factory returns a fresh, unadmitted handle;
it owns cleanup of any partial factory failure. The lease installs that handle's
cleanup owner before calling acquisition. Its operation guard covers the factory
as well as native effects. Settlement requires both no admitted handle and no
in-flight operation; an unfinished factory cannot advertise settlement.

Successful acquisition records a completed grant attempt. Repeated desired-on
requests allocate nothing while that original known handle remains owned. An
acquisition or retirement error retains the exact handle and failure, including
fatal errors which propagate unchanged. Uncertain handles cannot be reacquired or
replaced. Only that original handle's normal retirement return clears ownership
and admits a fresh successor. Reentrant mutation is rejected before changing state;
read-only settlement during an operation remains false. Native/backend calls are
serialized and non-suspending; an implementation must not wait for another call
back into this owner. The containing execution owner must isolate potentially
blocking native calls and retain/join them before releasing their context.

The lease retains at most one handle record, its last failure and an operation
flag, plus the caller-owned factory. The caller bounds failure/factory objects and
owns the containing lease lifetime and any external views. Backend callbacks and
borrowed native objects must be cut off/fenced by the backend retirement contract.
Configured native grant/deadline/capacity limits remain backend contracts; a grant
is not an unlimited wake promise. No timeout, cancellation request, diagnostic
held flag or reference count establishes actual retirement here.

Conservation requires every admitted grant/unknown obligation to have its exact
retained handle and cleanup sponsor. No second handle is allocated before original
retirement. Initialization starts empty; the factory guard precedes allocation;
admission precedes acquisition; failure preserves that owner; actual retirement
alone clears it. Conditional eventual cleanup requires the sponsor to keep calling
retirement and the backend eventually to supply actual original fence evidence.
Missing evidence keeps the obligation owned and funded. A caller stopping its
coroutine or losing foreground authority must retain the original native operation
and cleanup context; this pure owner does not provide native process/thread proof.

Permanent tests cover partial acquisition, factory failure, repeated desire,
fresh successors, release failure, fatal effects and reentrant factory/native
callbacks. An independent model checks all 1,296 four-step histories of desired-on,
stop and acquisition/retirement fault toggles, including failure after a simulated
grant. Its independent handle list supplies unconditional fixture cleanup. A
deliberate early-release control fails the permanent retirement/fatal regressions;
the factory-settlement regression was observed failing before its fix. These checks
prove bounded pure-owner behavior under the handle contract, not Android hardware
or native backend conformance.


## Foreground native service ownership

`StreamingPowerBinding` owns one opaque foreground claim and its policy observer.
Stopping it cancels that actual observer and withdraws only its original claim.
`StreamingPowerRuntime` owns the application handoff channel, one retiring native
worker plus one latest replacement, and one original idle-timer callback plus one
latest target. The manifest Application owns this runtime strongly for its process lifetime,
including outstanding handoffs and quarantined work after producer loss. There is
no weak-registry reset or second runtime for the same Application. Active bindings,
queued Binder capabilities and actual jobs borrow that original runtime.
All request/service/timer effects serialize on the main actor; native wake work
runs through `AndroidWorkerRun` in the private `:power` process. Independent camera,
microphone, speaker, display and metadata workers retain their separate owners.

The Android component remains private to the application. Its foreground
notification does not imply native lock ownership. A local Binder capability names
the exact original queued notice; immutable Intent fields must match that notice's
initial policy. Unknown, duplicate and stale capabilities cannot consume a newer
admission. Pending identity is checked before foreground promotion. A confirmed
never-admitted start/post rejection may settle that slot; an ambiguous exception
retains it, withdraws original authority and denies extra admission. Destruction
first detaches its original port, then fences its exact queued callback. Failed
cancellation keeps the slot until the original guarded callback arrives. An old
component or foreground producer cannot revoke a replacement owner.

The current original receiver publishes immutable request/task views. A lazy
worker waits for actual current delivery before native admission. Same native
tasks retain the original process; changed tasks cut off the old view and enter
the existing retirement lane. Late creators remain owned and joined. Original
native readiness must match its task and opaque stamp. Wrong readiness or worker
loss cuts off that original owner before UI reporting; it cannot automatically
reacquire. Actual original Binder death and in-flight control settlement precede
replacement admission. A framework disconnect, canceled coroutine, stopped flag,
readiness event or elapsed timeout is not that fence. Missing proof retains the
original lane and funded cleanup sponsor.

`PowerWakeRun` installs the CPU/Wi-Fi leases before native acquisition, stops
without native IO on the Binder/UI thread, and attempts both original retirements
even if either fails. Its Android handle requires normal original API release;
an uncertain unheld acquisition cannot manufacture proof from `isHeld == false`.
Factory/acquire/release faults retain the exact handle. The containing original
private process provides the final process-access fence for blocked/unknown calls.
The existing CPU grant is bounded to four hours; a subsequent current policy
receipt after that budget renews through an actual retired predecessor. This is
not an unlimited CPU-grant guarantee. Wi-Fi uses low-latency mode from API 29 and
the previous high-performance mode on older supported SDKs.

Idle Wi-Fi grace requires matching actual acquisition readiness. Its original
absolute five-second due time survives repeated idle requests; USB, pause and
source/background loss bypass grace. Enqueue/cancel ambiguity preserves one
original callback and withdraws grace demand. Negative/overflowing clock values
fail closed; a bad queued callback clock is contained and reported on the main
loop. Duplicate old callbacks cannot acknowledge a successor or release it early.

Conservation pairs every queued admission/native grant or unknown obligation
with its original retained cleanup owner. Reuse requires original receipt/callback
fencing or actual worker retirement respectively, never logical completion alone.
The containing runtime settles with its Application/process; its bounded queues
and native grants retain their separate original settlement obligations.
Conditional eventual cleanup assumes native delivery/scheduling, original control
calls and OS process/token cleanup eventually progress. Selected cached SDK 27/34
framework bytecode shows local held-state changes after synchronous release calls;
it does not prove unrestricted system-server/kernel cleanup timing. Robolectric
checks real adapters against bounded independent fault models and joined local
worker cleanup; physical process, device wake and battery acceptance remain T914.
