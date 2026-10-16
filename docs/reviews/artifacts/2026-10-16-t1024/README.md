# T1024: revoke original helper publication before cancellation

Each stdout drain receives one original publication authority. Its owner revokes
before aborting work. Publication checks that authority inside the watch write
transaction, and revocation takes empty transactions on every affected channel
to wait out previously authorized updates. This preserves current values and
versions while preventing late mode, stream or FIFO notices from changing them.
A fresh owner uses a different immutable authority allocation; an old publisher
cannot be revived. The helper keeps the same watch state owner across restarts.

Asynchronous abort retains the original stdout task's actual completion and FIFO
lease. Replacement rejects before resources/spawn while the original child,
stdout task or its pending cleanup remains owned. Termination observes stdout
cleanup within the existing configured reap budget, preserving its certificate
when the wait expires. It never infers cleanup from elapsed time. Fresh successful
startup clears the earlier stdout failure latch. Whole capture/session dependent
retirement remains the separately tracked T1026 integration prerequisite.

Three permanent normal-suite behavioral regressions fail before the fix:
late original reports overwrite current state, replacement reaches its spawner
while original stdout cleanup is held, and reentry with a live helper reaches a
replacement spawner and changes its FIFO. The tests use real owned blocking tasks
and native children, not completed stand-ins. They retain original certificates,
verify abort/timeout cannot complete held work, release that exact work, and then
allow a fresh successful startup. Existing restart/shutdown/FIFO regressions pass.
Later assertions extend coverage to both still-owned and retiring stdout states;
no pre-existing assertion or regression was removed or weakened.

Fresh locked offline collection passes 752 host library and 453 binary tests;
seven pre-existing benchmark ignores remain unchanged. An independent installed
SDK collection passes 20 optional encoder helper tests. Verified joined native
LCOV covers all 35 helper/framing/publication production functions individually
at 80% or better. Publication functions reach 100%, stdout retirement waiting
82.35%, and startup 98.18%. Separate configuration reports retain their missing
counters honestly; only the verified joined result passes the whole scope.
Manifests, native source/object attestations, raw LCOV and per-function results
are included. Native objects remain under the two fresh
`/tmp/blent-t1024-evidence/coverage-{cli,sdk}/target` directories.

Source audit covers 77 authored production/test functions, maximum complexity 8,
using unchanged repository Sonar-rule definitions and cached pinned Rust parser.
Native SonarQube remains unavailable under T1019. SDK network was explicitly
disabled; no tool/package transfer, desktop action or installed-app update ran.

```sh
python3 /tmp/blent-t1024-evidence/scoped_rust.py coverage-cli
PATH=/tmp/blent-t1024-evidence/sdk-tools:$PATH \
  python3 /tmp/blent-t1024-evidence/scoped_sdk.py coverage-sdk
python3 /tmp/blent-t1024-evidence/merge.py
```

T1026 will carry dependent native proofs through whole session replacement;
T1027 supplies shared activity observations; T1015/T1016/T1009 compose runtime
display policy. These component results do not claim physical host/tablet
qualification, which remains T914.
