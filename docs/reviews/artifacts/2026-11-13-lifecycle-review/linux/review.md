# Linux lifecycle review

Reviewed HEAD `56250b92`. Review only: no production or test sources changed.

## Confirmed findings

### T1413: D-Bus activity backend loss never requests recovery

`host/src/activity/native.rs:265–277` catches original actor failure, publishes
unknown fields and exits without setting `Shared.replacement`. Owner loss and
stream closure return errors at lines 287–292; owner replacement is rejected at
lines 342–348. This preserves stale-owner exclusion but leaves no live observer.

`host/src/activity.rs:262–270` only requests replacement when the queue reports an
error or the replacement flag is set. The emptied queue therefore returns
`Ok(None)` indefinitely. `host/src/activity_follow.rs:176–183` enters its recovery
workflow only on polling failure. DPMS sets the flag in `activity/dpms.rs:170`;
the D-Bus adapter does not. This is an adapter gap in the T1343 recovery wiring.

After logind loss, supported power remains unknown and both display and auxiliary
demand remain false (`common/src/host_activity/policy.rs:41–56`). After compositor
owner loss, enabled display-power policy leaves display demand false. A new
healthy D-Bus owner cannot restore the original actor, and no fresh source is
automatically created. Unrelated observer failure or daemon restart may recover.

Required fix/regression: request fresh-source recovery after failed D-Bus actor
termination, retain original task/native settlement before replacement, and test
the actual `Following` + Linux D-Bus adapter across private owner release and
reacquisition. Require a fresh epoch/state, deny stale-owner updates and preserve
cleanup fences. Existing private owner-replacement coverage manually creates a
fresh source and does not exercise this automatic recovery integration.

Severity medium; effort small; one cohesive native adapter/recovery outcome.

### T1415: Capture pause-control failure exits the primary session permanently

Source trace: malformed ACK or control EOF returns an error from
`common/src/linux/capture_control.rs::Control::finish`; `pause_helper` propagates
it through `host/src/capture/display_pause.rs:21–25`, before codec reset.
`capture/quiescence.rs::quiesce_and_publish`, `supervision.rs::pause_capture`,
`capture_step` (lines 50/56) and `follow_capture` (line 22) propagate this error out
of `stream_frames`. This is different from the implemented recovery on resume
control failure at `supervision.rs:109–132`.

The production capture future returns that result (`host/src/session.rs:120–129`).
`session_supervision.rs:252–260` treats it as terminal, retires the attachment and
stops the remaining session workers. The primary runtime is created once at
`linux_main.rs:1782`; the daemon then waits for daemon shutdown at line 1867.
Neither the Linux monitor nor its direct-USB composition recreates the primary
runtime: direct USB retains its existing attachment/status/listeners. Reconnect
cannot restart its terminated capture/input workers.

Supporting fault observation: ran the unchanged existing
`t1015_actual_running_pause_retains_helper_and_resumes_fresh_codec_and_idr` test
in a private runtime. Created `hold-pause` before startup, waited for its private
helper's `pause-requested`, then created `foreign-ack` and `release-pause`.
The two recorded requests are active sequence 1 and paused sequence 2. The test
failed waiting for the quiescent/header-reset condition. See
`pause-ack-fault.log` and `pause-ack-requests.txt`. The assertion timeout alone is
not proof of permanent failure; the propagation and primary-orchestration trace
above establish that consequence. No modified test or full-daemon fault trial
was executed. Control EOF follows the same error propagation but was not injected.

Required fix/regression: on failed pause, terminate the original helper and retain
helper/stdout/encoder ownership until actual native retirement, clear codec state,
and keep the supervisor available for later permitted demand. Add permanent
private-helper EOF and malformed-pause-ACK regressions: no replacement capture
while paused, fresh codec/IDR after resume, no reuse before original settlement.
Keep ordinary shutdown terminal. Severity medium; effort small; one cohesive
pause-recovery outcome.

## Examined scope and retained invariants

- Linux ADB monitor inventory, identity revalidation, preparations, forwarding,
  admission and retirement: `monitor.rs`, `monitor/{preparation,forwarding,
  admission,retirement,routes,usb_display}.rs`, and `discovery.rs`.
- Shared display route owner/worker: `usb/routes.rs`, `usb/routes/worker.rs`.
  T1344/T1346 retain creator/removal debt; Linux retirement waits route settlement
  and original session completion before returning the display ticket. T1410's
  optional original session proof remains part of route lease retirement.
- Linux activity sources, power adapter, D-Bus actor, DPMS recovery branch,
  follower/recovery and policy: `activity.rs`, `activity/{native,power,dpms}.rs`,
  `activity_follow.rs`, `activity_follow/recovery.rs`, and common activity policy.
- Capture activity/attempt supervision, helper startup/retirement, pause receipts,
  and session supervision: `capture.rs`, `capture/{activity,supervision,helper,
  display_pause,quiescence,process}.rs`, `capture/helper/activity.rs`,
  `session.rs`, `session_core.rs`, `session_supervision.rs`, `session_work.rs`,
  `registered_session/activity.rs`, `display_connections.rs`, and primary daemon
  startup/wait composition in `linux_main.rs`.
- Selected native helper sections: `host/evdi/capture.c` activity control/event
  loop, mode-buffer retirement and pause/resume; `capture_activity.c` and
  `capture_control.c`. These are source observations, not physical EVDI evidence.

The reviewed capture path retains original helper/stdout completion before helper
replacement, original encoder retirement before reuse, and exact pause ACK before
quiescence publication. Coalesced source stop/resume fences remain observable.
Native C pause handling applies control before queued pixel callbacks and retains
buffers while waiting for writer quiescence. No additional confirmed leak,
double-release or stale-identity bug was found in these inspected paths.

## Checks executed

All commands used offline locked Cargo with `accessory-transport`; existing test
thread configuration was preserved. Results are selected-test results, not
coverage percentages or platform-support claims.

- Library private D-Bus owner-replacement regression: 1/1 selected test passed.
- Library `activity_follow::recovery_tests`: 3/3 selected tests passed.
- Binary `capture::activity_tests`: 19/19 selected tests passed, including joined
  native pause/resume, coalesced fences and held original helper acknowledgments.
- One deliberate fault run of an existing private capture fixture: 1/1 selected
  test failed after malformed pause ACK, as described above.
- Read-only post-check: user service remained active/running with MainPID 608431;
  no process command line for the private fault runtime remained.

No suspend/resume, USB unplug/rebind, driver reload, input injection, service stop,
tablet change or configuration change was performed. No physical suspend/resume
or repeated-device reconnect campaign was executed, and finite private tests do
not establish native hardware acceptance or unrestricted lifetime safety. Existing
blocked Windows acceptance and declined performance/stylus/macOS work remain out
of scope. Parent recorded the findings in TODO.md as T1413 and T1415.
