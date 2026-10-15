# T1021: retain native monitor while capture is paused

An optional private sequenced-packet endpoint starts the native capture helper
paused. Its independent C wire contract matches T1023's fixed nine-byte records.
Only consecutive nonzero sequences and strict booleans are accepted. Invalid,
truncated, replayed, descriptor-bearing and closed channels stop the helper.
The Linux adapter owns descriptor validation and nonblocking command/echo I/O.
Legacy startup without the optional endpoint keeps its existing active behavior.

The event loop applies control before queued native callbacks. Pause gates pixel
requests, grabs, conversion and publication while preserving monitor, mode and
registered framebuffer. It invalidates the FIFO generation once, then observes
the original writer lease until release. The bounded wait may expire without
acknowledging or freeing anything. A blocked echo remains pending. Resume marks
all damage, grabs fresh pixels before any new request and seeds unchanged images.
Shared-raw producer pause is separate from encoder reader/process retirement.

Permanent normal-suite tests cover independent fixed vectors and bounded
mutations, unsupported descriptors, channel errors, replay, held actual frame
leases, backpressure, queued callbacks, FIFO/shared-raw behavior and descriptor
cleanup. Thirty-two real SCM_RIGHTS deliveries with valid payloads are rejected
without increasing aliases of the original descriptor. Rust exec/socket handoff
drives sixteen transitions through the actual C capture event loop with fake
EVDI calls. Worker assertions check buffer/monitor identity and one fresh grab
before each resume echo, with no intervening paused requests or grabs.

The permanent pause test fails against the original capture implementation at
`g_conversion.last_jobs == 0`, after initializing that diagnostic counter to zero.
It passes with activity gating. `red-native-final.log.gz` preserves this behavioral
failure. An earlier feature compilation failure and a ptraced-sandbox LeakSanitizer
restriction were diagnostic only; neither is used as behavioral evidence.

The full locked offline native collection passes 74 tests across conversion,
helper, independent module linking, frame retirement and Rust/native integration.
ASan/UBSan and existing race checks pass. One added raw-channel error case then
supplies the missing coverage of the new channel orchestrator. The focused run
passes its permanent test; it is deliberately not reported as whole-helper
coverage. `merge.py.gz` verifies identical production source hashes and function
inventories before joining raw GCC counters. All 239 authored native production
functions independently exceed 80% (minimum 81.25%). Separate manifests, test
logs, reports, compiler invocations, native notes/counters and raw gcov JSON are
retained for both immutable runs. Only test coverage changed between them.

Arch and portable release fixture suites pass seven tests. Permanent assertions
verify both new native modules reach compiler arguments. Native Bash DEBUG
locations cover all five executable commands in the changed Arch `build`
function. Makefile, release and Arch builds all include the new translation units.
No real build container, package manager, fetch, download or live device ran.

Scoped source audits use unchanged repository Sonar-rule counting definitions
and cached pinned tree-sitter parsers: 144 C functions, maximum 9; 65 Rust test
functions, maximum 4; three changed Python test helpers, maximum 4. Arch `build`
has complexity 1 under the approved Shell count. Scoped Allman formatting uses
installed clang-format 18.1.3. This is not native SonarQube evidence; T1019 remains.

```sh
python3 /tmp/blent-t1021-evidence/coverage.py coverage
python3 /tmp/blent-t1021-evidence/coverage.py coverage-boundary focused
python3 /tmp/blent-t1021-evidence/merge.py
```

The first collection intentionally exposes its missing channel-error coverage;
only the verified joined report passes every function. Runtime composition
remains T1015/T1016/T1009, physical qualification T914. T1025 records the redundant
first-tick fallback grab found during review; this component does not claim that
later active-loop pacing is optimized. T1022/T1024 retain stdout framing and
publication prerequisites. No desktop setting, monitor connection, tablet or
installed app was changed.
