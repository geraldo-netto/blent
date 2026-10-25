# T1254 offline startup-exit review

The retained evidence establishes one worker-process death during the October 21
reload and a later live replacement. It does not establish the cause of that
death. Owner decision 2026-10-25: remove T1254 as irrelevant and end this
investigation. The TODO row is removed; the historical evidence is retained.
The cause remains unverified. No application defect,
expected cancellation, verified fix or native power-policy acceptance is claimed.

The source observations are unchanged in
[the original archive](../2026-10-21-reinstall/T1254/README.md).
`review.json` records archive/member and reviewed-source hashes. Relevant saved
events, in their respective device/host clocks, are:

| Time (CEST, 2026-10-21) | Observation | Source |
| --- | --- | --- |
| 13:38:38.486 / .499, device | Earlier display 16371 and power 16382 exits: reason 2, status 9 | `android-after.txt` |
| 13:38:40, host | Explicit host-service stop and clean shutdown logs | `host-reload-timeline.log` |
| 13:39:10.491, device | Main process 31445 exits: user-requested force stop | `android-after.txt` |
| 13:39:13.043, device | Power 20161 exits: reason 13, status 0, `Died when creating service` | `android-after.txt` |
| 13:39:14.013, device | Metadata 20205 exits: reason 2, status 9 | `android-after.txt` |
| 13:39:14–15, host | Host-service start, connection and tablet-launch command complete | `host-reload-timeline.log` |
| Later snapshot | Main 20068, display 20208 and replacement power 20306 alive | `android-foreground.txt` |

The original workflow orders force-stop, incremental install, MainActivity start
and host-service start, but does not separately timestamp the Android commands.
Device time is approximately two seconds behind host time. Those bounds do not
identify which command, service transition or system action caused the death.
The saved process-events member has only older process-start events, not a
complete trace of this reload. The install and activity-start success responses,
later process liveness and render acknowledgements cannot supply the missing
causal evidence. Existing exit-info contains no Java/native crash stack for 20161.

The focused code review covered `PowerWorkerScope`, `ScopedPowerRun`,
`ResourceLane`, `AndroidWorkerRun` and `MediaWorkerService`. Current contracts keep
the original lane owner through execution join and successful native retirement;
failed retirement cannot admit a successor. Android binding callbacks validate
the original generation, and retirement keeps the binding until required Binder
death observations and cleanup settle. Worker service unbind/destruction invokes
process termination. These source paths describe possible lifecycle behavior;
the archive does not identify which paths PID 20161 actually executed.

Existing permanent tests include `PowerWorkerIntegrationTest` (replacement waits
for original cleanup), `PowerServiceLifecycleTest` (stale envelopes) and
`PowerServiceQueueTest` (queued-callback retirement). Their Robolectric SDK 27/34
contracts do not reproduce or attribute this API 36 native observation. No new
behavior is changed or diagnosed, so no artificial regression or new coverage
claim is introduced. A subsequently confirmed defect still requires a permanent
failing-before/passing-after regression and native cleanup evidence.

No physical campaign, package reinstall, device/process/service control, network
transfer or production change was performed. The archive is read-only, extracted
members are consumed in memory, and no background observer or retained native
handle is created by this review. PID 20161's recorded death is actual native
exit evidence; the saved data lacks its acquisition/release counters and cannot
establish every resource's cleanup or power-policy correctness.
