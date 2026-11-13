# T1441: direct OSK restoration lacks a process retirement sponsor

Confirmed on the frozen post-T1433 library, before any T1441 production change.
The input-owned path observes OSK operation completion through its admitted input
lifetime. Direct `osk::restore` has no equivalent sponsor: it acquires the private
operation gate/pool, whose proof is not registered with process retirement.

`linux_main.rs:1900` awaits direct restoration during normal daemon shutdown.
`osk.rs::restore` acquires the gate and returns when `restore_unlocked` returns.
`native_commands.rs::ObservedCommands::run` retains exact child/collector
observations, but its command deadline may return an error before those originals
settle. Dropping the OSK operation transfers those proofs and the serial guard to
its private two-slot pool. `runtime_owner::run` drains PROCESS, not that pool.
Consequently a late original restore command can survive daemon process exit.

Two safe isolated probe cases passed their counterexample assertions:

| Case | Runtime returned before release | Exact original child still alive | Calls before release |
| --- | --- | --- | ---: |
| Cancel direct restore after native child starts | true | true | 1 |
| Await direct restore through ordinary command deadline | true | true | 2 |

The deadline case first completes one private backend-selection probe, then holds
the actual restore set-property command. There is no caller cancellation in this
case. The ordinary five-second command deadline returns failure; the process entry
then returns while the original command is still alive. The retained backup is
recovery metadata, not evidence that this original native command has stopped.

`result.log` contains the actual child observations; `probe.log` records 2/2 probe
tests. `metadata.json` records source and object SHA-256 identities and all 5/5
probe function complexities (maximum 4). The probe includes the permanent
`host/tests/input_commands/fixture.rs` through a repository-relative path; no
shared fixture was changed. Compilation used rustc 1.100.0 `--test -C
instrument-coverage`, the current frozen T1433 rlibs, and its existing dependency
search paths. Exact compiler arguments, binary, frozen rlibs and raw profiles
remain outside the repository under `/tmp/blent-direct-osk`.

The fixture compiles a private fake busctl plus the fail-closed uinput/kill shim.
PATH/HOME/XDG directories are private; no real bus, device or user configuration is
used. The shim refuses to terminate only its exact original child. After observing
the failure, each case releases that command, joins its process-entry thread and
waits for the exact child PID to disappear before asserting the counterexample.
The deadline case's first probe is already complete before the held operation.
These are direct production API executions plus the verified daemon call chain;
the full daemon and live native backend were not run.

Resolution requires a bounded admitted process/daemon sponsor for direct
restoration, retained through original child/collector settlement on success,
timeout, cancellation and caller loss. Shutdown restoration must use a sponsor
admitted before process cutoff rather than reopen admission afterward. Keep the
input-owned cleanup path and its original input sponsor intact. Add permanent
normal-suite regressions before the fix for both cases and for the late-shutdown
call; retain original outcome/backup policy and exact native ownership.
