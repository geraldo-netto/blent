# T1030: task results do not release dependent cleanup

Owned task join results remain observable independently of resource retirement.
When dependent leases remain, the original admission transfers to the existing
isolated cleanup worker and stays incomplete until destruction actually finishes.
The asynchronous drop path first observes the actual task join, then uses that
same isolated cleanup mechanism. Blocking lease destructors cannot stall the
shared retirement observer or unrelated admitted work. Without dependencies,
already joined tasks still release immediately. Success, error, panic and
cancellation results retain their existing meaning.

Helper wait preserves the exact stdout certificate before consuming its task
result, preventing restart while post-join cleanup remains live. X11 startup
stores its original creator certificate separately from the consumed task handle.
Initial and repeated joins wait for that certificate, including cancellation
between join and dependency cleanup; child retirement remains independent.

Four permanent normal-suite regressions cover joined success/error/panic with
held leases, destructor entry, held original capacity, independent sibling
retirement, helper replacement after stdout join and canceled/repeated startup
joins. Before the respective fixes, decisive failures are:

- `T1030 joined result released live dependent leases`
- `T1030 blocked dependency stalled sibling retirement`
- `T1030 joined stdout lost original cleanup before restart`
- `T1030 startup join released live dependent cleanup`

The startup cancellation regression uses an explicit observer-start barrier;
earlier unsynchronized diagnostics do not serve as behavioral evidence. Every
fixture releases its original held cleanup before checking results, and the
tests remain in the normal suite. The first broad run also exposed the existing
T1013 unsupported-program retirement assertion. That assertion remains unchanged;
the adapter now waits for actual creator completion. Its diagnostic failure log
is retained separately and contributes no final counters. No regression or
assertion was removed, skipped or weakened.

Fresh locked offline collection against final identical sources passes 761 host
library and 454 binary tests, with seven pre-existing benchmark ignores unchanged.
An independent installed-SDK run passes 21 optional encoder helper tests. Joined
native LCOV covers all 49 owned-task, helper and X11 adapter production functions
individually at 80% or better. New lease-retirement paths and affected helper/X11
waits reach 100%; the task join observer reaches 91.67%. Separate configuration
reports preserve their missing counters; only the verified joined result passes
the whole scope. Immutable manifests, native object/source attestations, raw LCOV
and per-function results are retained. Native objects remain under
`/tmp/blent-t1030-evidence/coverage-{cli,sdk}-final/target`.

Source audit covers 101 production/test functions, maximum complexity 8, using
unchanged repository Sonar-rule definitions and cached pinned Rust parser.
Native SonarQube remains unavailable under T1019. SDK networking was explicitly
disabled; installed tools and caches supplied all dependencies. An initial
permission-review timeout was bypassed for thread-only tests inside the sandbox;
necessary full private-IPC coverage runs were subsequently permitted. No live
device/desktop or installed app was changed, and no transfer occurred.

```sh
python3 /tmp/blent-t1030-evidence/scoped_rust.py coverage-cli-final
PATH=/tmp/blent-t1030-evidence/sdk-tools:$PATH \
  python3 /tmp/blent-t1030-evidence/scoped_sdk.py coverage-sdk-final
python3 /tmp/blent-t1030-evidence/merge.py
```

Linux core registration remains T1028, auxiliary registration T1029, and activity
propagation/integration T1027/T1015. These resource ownership results do not claim
physical power/display qualification.
