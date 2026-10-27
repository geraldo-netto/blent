# T1078: SDK-correct null-binding fixture

A worker fixture called an API 28 callback on API 27, causing a delayed NoSuchMethodError. SDK-appropriate callbacks fixed the hosted failure; the complete 2,234-test collection then passed.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The clean T1011 full suite passed 2,231 cases but its next pairing UI case reported
an uncaught coroutine exception. The retained suppressed stack identifies the
original producer: ConnectionWorkerContext invoked ServiceConnection.onNullBinding
on API27, although that interface method starts at API28. The worker's original
IO coroutine raised NoSuchMethodError; a later Compose test reported it.

The new permanent synchronous fixture regression fails with that exact linkage
error on API27 and passes on API34 before the change. Afterward it passes on both
SDKs: API27 receives the nullable connected callback, and API34 receives the
dedicated null-binding callback. Existing production rejection handling is used
unchanged. The test also requires exactly one rejection callback, no Binder link
or START transaction, and one explicit fake unbind. No real service is acquired.

All 60 focused original worker/retirement and pairing UI cases pass. A fresh clean
offline Android collection then passes all 2,234 cases with zero skips, failures
or errors. Source hashes and the original red/green XML/logs are retained. The
source audit covers 189  fixture/test functions, maximum Kotlin PSI 1.9.20 estimate 6;
native SonarQube remains T1019. No production function changed and no cleanup,
timeout, assertion or exception filter was weakened. Fake callback state is
bounded to the invocation; original native worker fences remain exercised by the
unchanged suite. Physical API27/28 qualification remains T863.

</details>
