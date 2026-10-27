# T1028: original Linux capture proofs survive session stop

Linux capture records helper, encoder and reader proofs before fallible handoffs. Whole-session Stop cannot lose original native cleanup; scoped joined counters do not certify broad SDK or physical behavior.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The Linux session binds its independent recorder before exposing a fresh capture
manager. Helper, stdout, CLI/GPU encoder child and encoder reader completions
register immediately after native creation, before another fallible step or
await. The isolated encoder records its child before pipe extraction/task
admission, and its reader before returning ownership. Its output context retains
the original reporting lifetime while native creation remains possible.

Eight permanent normal-suite regressions use actual owned children/tasks and
held original dependency cleanup. Six traverse portable session construction,
Linux capture ownership and whole-session stop; two exercise the isolated native
worker adapter, including failed reader admission after child exposure. They
cover helper/CLI child cleanup, stdout/reader post-join cleanup and canceled or
failed helper handshake. Each releases its original cleanup and waits for the
same completion; none treats a parent result or elapsed deadline as retirement.
Before forwarding, retained tests fail with `T1028 session released original
native cleanup`, `T1028 missing original encoder proofs` and
`T1028 post-spawn failure lost original child`. Red logs retain those results.

Fresh locked offline native collections pass 761 library and 460 CLI binary
tests (seven pre-existing benchmark ignores). The installed SDK passes all 58
capture and nine isolated-encoder tests, including FIFO/shared-raw and actual
production worker death/restart fixtures. The SDK runner first builds and invokes
the real instrumented executable with `--help`, then collects encoder and capture
profiles in the same exclusively created target. The worker needs this executable
alongside its test binary. Raw counters, matching full source manifests and
native object attestations are retained. Joined counters cover all 89 scoped
production functions individually at >=80%; minimum 81.25%. Forwarding, worker
spawn and helper startup reach 100%. Separate configuration reports preserve
missing native counters; only the verified joined report passes the full scope.

The unchanged repository Sonar-rule source audit covers 145 production/test
functions, maximum 9, using cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2.
Native SonarQube remains unavailable under T1019. No tool was downloaded.

An initial broad SDK diagnostic ran without first building its worker and also
encountered unavailable private-bus/namespace prerequisites already tracked under
T835. Its failed log is retained separately and supplies no passing counters.
After preparing the executable, every scoped capture/encoder regression passes.
The default-feature host suite passes its native private-IPC tests. No broad SDK
suite pass, native hardware qualification or power improvement is claimed.

```sh
python3 /tmp/blent-t1028-evidence/scoped_rust.py coverage-cli-final
PATH=/tmp/blent-t1028-evidence/sdk-tools:$PATH \
  python3 /tmp/blent-t1028-evidence/scoped_sdk.py coverage-sdk-ready
python3 /tmp/blent-t1028-evidence/merge.py
```

Native objects remain in the corresponding `/tmp/blent-t1028-evidence` target
trees. SDK networking was disabled. No installed application, live display,
lock/suspend state or device was changed; no upload or repository push occurred.
Auxiliary retirement remains T1029; activity source/composition T1027/T1015.

</details>
