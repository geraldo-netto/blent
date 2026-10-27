# T1228: preserve original registration cleanup after admission failure

Failed resource admission now requests cleanup on the original socket before exiting. Native-socket regressions and a physical pending-offer expiry passed without requiring broker restart; broader media acceptance remained incomplete.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The physical camera offer was left pending beyond the local30s admission
deadline while inspecting pairing. The registration thread returned TimedOut,
closing its IPC before requesting Unregister or receiving original Retired.
Ownership correctly remained funded, but only original broker exit could then
settle the retained listener. This blocked subsequent clean media acceptance.

The permanent native-socket regression was added first. It failed with actual
UnexpectedEof instead of the required Unregister. After the fix all20 normal
registration tests pass, including failed/stalled cleanup, repeated close,
original broker death, late proof, blocked native work and deadline preservation.
On failure the original thread now marks the failure and makes one cleanup
exchange on the same original socket. Existing absolute cleanup deadline stays
unchanged. No reconnect, replacement owner or indefinite retry is introduced.
Failure remains visible even when exact remote retirement subsequently succeeds.

Raw LLVM counters from a fresh instrumented target measure the changed drive
function at12/12 executable lines. Exact executed object/source identities and
raw LCOV are retained. Rustc1.98.1/cargo-llvm-cov0.8.7; source complexity estimate2,
not native SonarQube evidence. Native Sonar remains unavailable; no download.

The current production host was rebuilt. An actual pending front-camera offer
on the authenticated tablet expired and the original CLI exited without the
previous retirement warning or broker restart. Existing speaker session and its
clocked helper stayed alive; owner separately confirmed hearing its test tone.
Physical media acceptance remains T858/T860/T859; these checks do not complete it.

The factory retains one thread and original duplicated listeners per admission;
the native reaper requires both actual thread exit and exact remote retirement.
The original broker pidfd remains its fallback exit observer. Timeout, EOF and
logical Stop grant no release. Success now obtains the already required original
Unregister/Retired proof before thread exit. Rejection, failed send, cancellation,
cleanup timeout, broker loss and stalled native execution retain existing bounded
capacity and cleanup sponsors. Conditional eventual release still requires exact
remote proof or original broker death and actual local native return; no claim of
unrestricted native leak freedom follows from these bounded fixtures.

</details>
