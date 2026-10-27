# T883 bounded Android native session runner

Fixed native read, write and close workers keep blocking calls off the serialized Android session owner. Interoperability tests observe worker exit; a cleanup deadline alone never authorizes replacement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Independent endpoint, native I/O and run contracts connect the tablet session to
stock streams without placing native calls on the serialized protocol owner.
Three fixed workers own read, write/flush and close. Exact completion slots stay
bounded through consumption; a prompt conflated wake and independent supervisor
clock preserve progress and deadline observation when native operations block.

Offline validation with cached dependencies:

- 194 ordinary connection tests pass, including partial/zero progress, malformed
  counts/headers, duplicate/foreign completions, stalls in each I/O direction,
  close stalls/failures, startup failures and actual native retirement.
- Ten retained real Rust/Android TLS tests pass. The new API27/34 scenario uses
  actual StreamConnectionIo and ConnectionSessionRunner against the Rust host
  Session, exchanges display/control bytes, retires and replaces the generation,
  and waits for actual native read/write/close worker exit.
- All 38 inventoried production functions pass the per-function coverage gate.
  Raw JaCoCo evidence includes 75 executable JVM methods, constructors and getters;
  every method exceeds 80%, minimum 92.31%. T848 inventory gaps remain explicit.
- All 153 reviewed production/test functions have cyclomatic complexity ≤9.

Two permanent regressions failed before their fixes: a completion racing the
final retirement check falsely failed a clean stop on the next wake; interruption
of an idle native worker silently lost its direction without revoking admission.
Both pass with the same tests retained in the ordinary suite. Invalid-input tests
also cover extreme counts, invalid deadlines and 64 zero-progress iterations
without resetting the absolute frame deadline.

Evidence includes pre-run source inventory, source hashes, JaCoCo execution/XML,
per-function and raw per-method results, passing logs and the failing regressions.
No test exclusions or coverage thresholds were weakened.

This work does not implement the private Android service or prove real blocked
syscall/process termination. T884 retains private IPC/process supervision; T854
retains accessory permission/setup; T857 retains local endpoint pumps and actual
production display acceptance. Cleanup timeout requests process termination and
never authorizes replacement. No downloads, device installs, USB-mode changes or
network-configuration changes occurred.

</details>
