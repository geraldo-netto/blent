# Android display wake policy

Status: independent desired-policy values and framework value conversion are
implemented under T1059. Original native service ownership is T1060; Activity
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
