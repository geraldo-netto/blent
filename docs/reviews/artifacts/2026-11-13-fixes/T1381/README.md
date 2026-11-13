# T1381 — independent portable identity observations

Three permanent progress regressions fail against the pre-fix production code;
the fourth endpoint-reuse regression already passes and remains a lifecycle
control. All four pass after the fix. Three additional query-owner tests cover
deadline, absent/stale completion and task-failure paths. The final USB selection
passes 35/35 registered cases, including both T1378 regressions. Linux monitor,
discovery and shared scheduler selections pass 55/55, 8/8 and 1/1 respectively.

The portable adapter reuses the existing DeviceTasks implementation through the
host library. It owns at most four running query tasks; current inventory remains
limited to 256 endpoints. A disappearing endpoint cancels its queued work and
aborts its original running task. Running handles remain owned until collected;
at most four obsolete active tasks can coexist with 256 current queued entries.
No second query task for an endpoint starts before its original handle settles.
An inventory incarnation and its admission bit prevent old successful or failed
tasks from publishing into a replacement observation.

Canceling a polling future leaves admitted work in Network. Confirmed inventory
removals publish without waiting for other identities. Initial pending devices
are not admitted; failed or timed-out completed probes publish unknown and revoke
cached identity authority. Existing devices retain their last completed identity
while a replacement observation is pending. Each query has a five-second
observation deadline. Launch history is retained until initial probes settle,
using the existing bounded launch registry to preserve migration behavior.

Monitor shutdown stops and joins query tasks before route retirement. Drop aborts
remaining task handles. Query cancellation is not proof of native process exit:
NativeCommands retains original child/collector cleanup through its existing
bounded retirement owners and process-capacity admission. Native settlement and
eventual executor/cleanup progress remain separate obligations. These tests use
synthetic Commands or owned local shell fixtures, not physical ADB mappings or
Windows handles. T1382 retains the native Windows acceptance obligation. No
unrestricted lifecycle or native-platform claim is made.

`functions.json` records all 13 changed/new production methods plus all nine
unchanged scheduler methods after sharing its implementation. All 22/22 measured
methods exceed 80% executable-line coverage; the minimum is 84.62%, maximum
complexity nine. Coverage was rebuilt after the final source change before
combining the four selections. Tools: rustc 1.98.1, cargo-llvm-cov 0.8.7, project
syn 2.0.119 complexity frontend. The containing project-wide complexity result,
formatting, Clippy and Windows cross-compile logs are in the parent directory.
