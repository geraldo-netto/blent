# T1096: original actor notifications before longer waits

Native admission and trust writes notify their original actor before tighter budgets can block. Fault tests and a bounded virtual-timer model passed; notifications do not prove resource retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Accessory native-call admission now notifies its original actor after tightening
the absolute budget and before entering blocking work. It rechecks live admission
after notification, so a failing/stopping reporter cannot start the native action.
Setup trust mutation similarly publishes its tightened storage budget before the
write blocks; required durable revocation cleanup remains allowed after stop.
Admission's original cleanup Deferred registers a completion callback before start,
so its actual result wakes the original awaiting actor rather than waiting for a
poll. Notifications do not reset budgets, grant authority or establish retirement.
Timer frequency/defaults and actor wait policy remain unchanged under T1092/T918.

Four permanent normal-suite regressions fail before behavior changes and pass
afterward: asynchronous shorter native callback budget without notification,
notification failure admitting native work, blocked trust write without original
native-authority notification, and actual cleanup completion waiting for a frozen
poll timer. The storage fixture initially violated canonical sorted trust input;
it was corrected and separately shown failing for the intended missing notification
before any production fix. Raw original/corrected red and green evidence remains.
The native-thread case observes exact original authority identity, distinguishing
its notification from the UI queue wake or a late startup wake. The cleanup case
holds virtual time at zero, then also checks 64 late original hints after retirement.

A reusable test-only CoroutineDispatcher/Delay provides retained virtual timeouts,
with independent 64-entry scheduled and dispatched capacities, exact original timer
objects and cancellation cutoffs. Four permanent tests cover invalid/overflow times,
both capacities, cancelled coroutine delay cleanup, queued disposed callbacks versus
replacement identity, and all 64 six-flag cancellation histories around enqueueing
three original timers. A no-cutoff control fails both the identity case and finite
model; the original helper is restored before final native collection. This finite
model does not cover unrestricted concurrent scheduling or native device behavior.

Production ownership remains with the original native owners, cleanup task and
actor channels. One additional completion handler is owned by each original cleanup
Deferred until terminal completion, including error/cancellation; a late notification
still targets the same bounded conflated channel and safely fails after close.
The task result is checked independently and native/permission proofs still govern
retirement. An error or logical callback never frees original worker admission.
Partial native work remains with existing cleanup sponsors; conditional cleanup
requires those exact native workers/backend obligations eventually to settle.

Test helpers own only bounded JVM callback/context records. Timer disposal prevents
not-yet-started invocation and never frees/reuses an executing callback's context;
its original run stack remains the owner until return. Registry capacity is released
only after dispatch admission or disposal, with dispatched records independently
funded until drained. An executing callback is an additional single scheduler-owned
record. Original jobs/gates are released and joined on success and failed assertions;
faulty no-wakeup cleanup is sponsored by explicitly advancing virtual time in final
cleanup. No native handles are owned by the scheduler. Existing native-owner fixtures
retain actual JVM threads, while backend/device operations remain bounded fakes.
No unrestricted native safety/leak-freedom or physical USB acceptance is claimed.

Fresh clean offline JaCoCo collection passes all 95 selected notification, scheduler,
owner, admission, boundary and deadline cases without failures/errors/skips. All eight
changed production constructors/functions/callbacks measure 100% native line coverage
individually; synchronized inline bodies remain within their owners under the current
strict checker. Source/class hashes and raw XML/exec are retained. All 12 architecture
checks pass. Cached pinned Kotlin PSI 1.9.20 audits 638 functions/callbacks, maximum
repository Sonar-style source estimate 9 (finite cancellation-history test).
Native SonarQube remains unavailable under T1019. No broader suite repeat is claimed.

No network transfer, download, installation, restart, physical handling or push.

</details>
