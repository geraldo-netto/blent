# T864 control credential snapshot regression

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

`SessionCoordinator.applyToken` publishes the new control token before calling
disconnect. An old pending socket can open in that interval, pass the existing
current-socket check and read the newly published credential. `ControlSession`
now snapshots the token (including null) under its socket lock before opening
each connection. Authentication uses that attempt's snapshot.

The permanent `ControlCredentialTest` reproduced six failing cases on API 27/34
before the behavior change: replacement credentials, previously tokenless startup,
and null/malformed updates. The same assertions pass afterward, including a stale
open callback after replacement. The test remains in the normal unit suite.

All 32 focused credential, capacity, token-delivery and decoder-negotiation cases
pass offline. Four affected native method counters (two constructor variants,
sendAuthentication and connectWebSocket) have 100% executable-line coverage.
Production/test cyclomatic complexity is at most eight. This is a changed-method
measurement, not a claim of full ControlSession or project coverage. Logs, native
counters, source hashes and snapshot are retained here. No device install occurred.
