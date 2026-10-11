# T885 actual media-worker retirement

Framework binding invalidation stops execution but cannot prove process death.
The original Binder must report death or become non-alive. Failed retirement
retains its binding and native START/STOP/EXIT jobs; blocked IPC cannot outlive
successful retirement, even after process exit. Independent EXIT remains able
to run when STOP is stuck. Cancellation during START retains the native call.
Failed unbind retains the original failed cleanup result and cannot pass on retry.
No replacement native owner is authorized by a timeout or Ended event.

Permanent regressions are proven red before each behavioral fix: binding loss,
blocked STOP/EXIT, blocked START and failed unbind retry. The previous timeout
regression remains permanent, strengthened to require zero unbinds before actual
death instead of its contradictory early-release expectation. Existing startup,
callback generation, overload, transport failure and late-binding tests remain.
Invalid cleanup limits and exact valid endpoints are covered on API 27 and 34.

All 924 offline connection, media-worker, camera, audio/display binding and resource
retirement tests pass. All 30 scoped production methods meet individual native
JaCoCo 80% executable-line coverage; all three authored constructors pass. All
39 reviewed production/test functions have cyclomatic complexity at most nine.
Native mappings, source inventory, hashes and red/green evidence are retained.
Counters are scoped to AndroidWorkerRun; this is not whole-project coverage.

Robolectric uses synthetic Binder liveness/death and blocked native calls. This
establishes adapter ownership, not real camera/codec or physical USB acceptance.
Production camera remains disabled pending T942/T943/T944/T932; joined hardware
acceptance stays T858. No downloads, installation or live device changes.
