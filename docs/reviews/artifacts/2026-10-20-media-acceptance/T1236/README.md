# T1236 — bounded native USB failure diagnostics

USB failure logs expose bounded phase, identity and error-kind fields without sensitive payloads. Broker tests passed; improved diagnostics alone did not fix software reconnect.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The native server discarded link and link-event errors, leaving failed physical
pairing/reconnect with no diagnostic. Its standalone binary also installed no
tracing subscriber. Two permanent normal-suite regressions fail independently
before their fixes: original server Input::Link emits nothing, and an existing
global subscriber does not prevent native entry work. Both pass after the fix.
Fixed phase-label coverage remains in the same suite. The final native broker
cohort passes236 cases, plus the instrumented binary startup case. Formatting
and whitespace checks pass. No external package, network transfer or push occurs.

Failures now report exact local connection ID, fixed phase enum, I/O ErrorKind
and timeout boolean. They never format the underlying error, certificate, token,
credential or arbitrary peer/native text. The permanent sentinel test proves
payload omission while preserving identity and ConnectionReset classification.
The binary initializes a warning-level, non-ANSI stderr subscriber before any
runtime/device ownership. Initialization failure returns a fixed diagnostic.

Six changed production functions have individually measured native executable
line coverage:100%,100%,95.24%,90.91%,100%,87.5%; see functions.json. Entry counters
are exported independently from the binary test object; library counters use
only profiles newer than its final rebuilt test object. Old949xxx profiles are
excluded. Unrelated old library mappings inside the entry object are not used.
The entry error callback is executed by the permanent initialization-failure
control. LLVM LCOV, exact object/source identities and logs are retained.
Complexity estimates are1–6, including test helpers/callbacks; these are not
unavailable licensed SonarQube measurements. No exclusions or suppressions added.

The anyhow context owns only a fixed enum while preserving the original error
chain. Diagnostic fields borrow it synchronously, retain nothing, and expire
before controller failure handling returns. The process owns the one global
subscriber until exit. Failure handling retains all original native task/helper,
route and retirement obligations; logging grants no authority, cancels no
additional worker and does not replace actual stop/fence evidence. Acquisition,
cancellation, timeout and late/unknown cleanup remain governed by the retained
original controller/pool contracts and the unchanged native lifecycle cohort.
Conditional eventual cleanup still requires original native return/exit and
required retirement proof; this diagnostic change proves no unrestricted native
leak freedom. Unknown obligations remain funded.

T1237 separately tracks the real Android invalid approval-ready marker observed
after software reconnect. A physical USB cycle permits fresh authentication and
camera/audio delivery; that does not establish the software reconnect fix.

</details>
