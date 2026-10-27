# T1029: idle-controller retirement survives reader completion

Idle-controller completion now survives reader-result consumption and canceled waits. Replacement requires exact task and dependent cleanup; synchronous file responsiveness remained separate work.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Idle controllers now use original owned-task admission. Their same completion
registers with the enclosing scope before handle exposure, and the task retains
its dependent lease until actual join and isolated cleanup complete. Existing
lease revocation, timing bounds and successor-token behavior are preserved.
Startup capacity exhaustion logs failure and leaves compatible cadence without
creating a controller. Task work readiness is exposed separately from original
resource completion; existing held-lease regressions assert that distinction.

Encoder readers retain their original idle completion before transferring its
handle into the packet loop. Successful retirement waits for both reader result
and original idle cleanup, including a reader whose result was already consumed.
The existing configured encoder-stop budget covers the whole wait. Cancellation
and deadline expiry keep the same certificate and cannot grant replacement; a
later retry succeeds only after actual original completion. Native worker builds
carry no idle controller and preserve their existing lifecycle behavior.

Four permanent normal-suite regressions cover an actual controller stuck in a
callback across abort, held original post-join idle cleanup after reader result,
production startup proof registration and startup-capacity fallback. The reader
regression also verifies canceled waiting, configured deadline failure and retry
after exact cleanup. Decisive pre-fix failures are:

- `T1029 idle controller lost original task retirement`
- `T1029 joined reader ignored original idle cleanup`
- `T1029 deadline must reject original idle cleanup`
- Missing original retirement before a startup handle is exposed.

After the final oracle additions, removing proof forwarding and dependent waits
again makes the retained tests fail. Restoring forwarding/waits makes those same
assertions pass; capacity rejection remains successful throughout. No regression
was removed, skipped or weakened. Existing timing/timestamp, token preservation,
invalid cadence/path and bounded-history tests remain in the normal suite.

Fresh locked offline collections pass 768 library and 472 CLI binary tests, with
seven existing benchmark ignores unchanged. Installed-SDK validation passes 59
capture and nine isolated-encoder tests using the instrumented production worker.
Seven contract/portability checks pass. Matching source/object attestations and
raw LCOV cover all 41 task/idle/encoder production functions individually at >=80%.
Separate configuration reports preserve missing counters; the verified joined
report passes the complete scope. Source audit covers 72 production/test functions,
maximum complexity 8, using unchanged repository Sonar-rule counting and cached
tree-sitter-rust 0.24.2/tree-sitter 0.25.2. Native SonarQube remains T1019.

```sh
python3 /tmp/blent-t1029-evidence/scoped_rust.py coverage-cli-final
PATH=/tmp/blent-t1029-evidence/sdk-tools:$PATH \
  python3 /tmp/blent-t1029-evidence/scoped_sdk.py coverage-sdk-ready
python3 /tmp/blent-t1029-evidence/merge.py
```

Native objects remain in the corresponding `/tmp/blent-t1029-evidence` target
trees. Idle file effects remain synchronous under T1034: a held `LEASE_FILES`
mutex can still occupy a control worker. That independent responsiveness fix is
next; activity propagation/composition remain T1027/T1015. No download, install,
transfer, push, live device/display change or application update occurred.

</details>
