# T1440: catchable signals bypass actual Linux Wi-Fi CLI ownership

Confirmed medium-severity behavior defect; proposed effort small/medium, one
cohesive CLI cancellation/retirement integration. Review only: no production or
permanent test changes in this artifact.

`host/src/linux_main.rs:93` wraps CLI work in `runtime_owner::run`, but the Wi-Fi
branch at line 107 directly awaits `wifi::linux::setup`. The SIGINT/SIGTERM listener
is constructed only by `run_daemon` at line 1648. Thus the Wi-Fi CLI retains default
termination behavior for ordinary catchable signals. The Wi-Fi exit guard
(`host/src/wifi/attempt.rs:70–78`) can drain an original owner only when stack
unwinding/return reaches its destructor.

The actual setup operation starts its endpoint attempt before persisting the
address (`host/src/wifi/policy.rs:98–110`). Terminating the CLI while its original
connect child is pending bypasses both the caller's cancellation handoff and the
exit guard. The original command can produce its endpoint afterward, leaving no
retained owner to compensate or save it. The code comment excluding forced OS
termination does not establish an intentional exclusion for user-facing catchable
SIGINT/SIGTERM; this finding does not require cleanup after SIGKILL.

## Reproduction

Executed the real, frozen instrumented Linux executable built during T1432:
`/tmp/blent-t1432-coverage/blent-bin-False`, SHA-256
`cc24402247a09aa92631725a387af41cc313366b5cb0ba3b71c63a5e148490e3`.
Its source/object manifest is `../T1432-coverage.json`; the CLI/Wi-Fi paths reviewed
here were unchanged by that fix. Working HEAD at report time was
`4ec3e032dcdf9c443e32411bf4ca7e8630ada5c6`.

```text
/usr/bin/unshare --user --map-root-user --mount --pid --fork --mount-proc --net /usr/bin/python3 docs/reviews/artifacts/2026-11-13-lifecycle-round3/wifi-cli/signals.py /tmp/blent-t1432-coverage/blent-bin-False
```

The private fake `adb` reports one USB tablet and its package/IP, then holds the
original `connect` command before publishing a file-backed endpoint. It never
contacts an ADB server or opens a device. Each case has private HOME, XDG runtime
and config directories, and PATH contains only the fake adapter. The wrapper runs
as PID 1 in a private namespace and reaps the orphaned original child after release.
Namespace teardown is the final descendant-containment boundary on fixture failure;
reaping the direct CLI child alone is not claimed to contain descendants.

`signals.log` records 3/3 completed scenarios:

| Scenario | CLI exit | Signal caught | Late endpoint | Saved address | Disconnect |
| --- | ---: | --- | --- | --- | --- |
| Normal control | 0 | n/a | present | yes | no, intentionally retained |
| SIGINT during original connect | -2 | no | present after CLI death | no | absent |
| SIGTERM during original connect | -15 | no | present after CLI death | no | absent |

Both signal cases target the actual CLI PID. The fixture confirms the original
creator remains alive after CLI death, releases it, waits for its effect and reaps
it. This models a delayed original creator, not a whole-terminal-process-group
signal or a real remote ADB server. No external endpoint or physical tablet is
needed to establish the missing entry-point handoff.

## Required resolution

Integrate catchable cancellation at the actual Wi-Fi CLI entry before admitting
work. Preserve one authoritative stop owner, shut off further admissions, and hand
cancelled original endpoint work to retained cleanup before process exit. Preserve
normal success and unrelated validation/error reporting. Do not cancel arbitrary
startup owners without transferring their original obligations.

Add permanent actual-entry SIGINT/SIGTERM regressions before the fix, retaining
both delayed creation and held cleanup through the process fence; include normal
successful setup, failure/rollback and Wi-Fi-off controls. Generic `runtime_owner`
API regressions alone do not cover this entry-point omission. T1432's existing
`wifi_exit` cohort verifies graceful generic API cleanup and remains a clean
separate path.

`complexity.json` records all 6/6 authored Python functions, maximum 6, using the
repository Python-AST cyclomatic checker (interpreter version included). No native
Windows or broader CLI command signal behavior is claimed by this scoped review.
