# Android display wake policy

Status: independent desired-policy values and framework value conversion are
implemented under T1059; independent original-request admission is T1062 and original handle ownership T1063.
Original native service ownership is T1060; Activity
observation/window integration is T1061; joined acceptance remains T1058/T1011
and physical wake/power evidence remains T914. Current production Activity requests
still use Waiting. Value conversion alone does not establish native enforcement
of the new display states or a measured power saving.

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
a bounded request implementation. Production service integration remains T1060.
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

T1063 supplies the independent WakeHandle contract and WakeLease owner. Native
backend integration remains T1060. A factory returns a fresh, unadmitted handle;
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
