# T1192: bounded current-caller compatibility audit

The October20 removal campaign is complete for the named host/shared/Android
configuration, decoder, metadata and probe paths. Independent removal commits
retain permanent regressions and their own native counters:
T1193 encoder alias; T1194 profile_cache; T1199 decoder V1 and qualification
bypasses; T1203 obsolete V4L2 probe CLI; T1204 lensless unscoped ADB stop;
T1206 shell inventory V1. T1189 updated the actual native probe retirement API.
The audit introduces no new runtime behavior; two source comments now describe
current functionality rather than label it legacy. No artificial policy test.

Remaining named candidates were traced through actual callers and contracts:

| Path | Current purpose / evidence |
| --- | --- |
| stream.rs authentication-disabled admission; stream/negotiation.rs optional64-byte token | VideoTransport.open sends a saved DisplayRoutes token even while the host disables checks. A fresh current client can have no saved token. T267's enabled/disabled/enabled, missing/stale token and bounded malformed-prefix/idle cleanup regressions all pass (4). Removing either route breaks the current auth toggle. The prefix is fixed64bytes and completion bounded by AUTH_TIMEOUT; socket handler/select owns reader lifetime. |
| common model/storage/cli; split cli/backend | Default fields support a current fresh/partial settings file; no old field rename/migration or CLI compatibility alias was found in this bounded scope. T264 top/bottom and case/space parsing are current user-facing position choices, not old schema conversion. Current canonical storage and invalid values remain tested. |
| common gpu/model plus current encoder selection | Empty opaque adapter identity and vaapi_device represent current automatic/pinned Linux selection. Preserve actual current device choice rather than remove it based on a legacy comment. T727 current roundtrip/merge tests pass. |
| common negotiation, ControlSession, RemoteMetadata/host doctor | Current decoder schema2 requires active scope; shell schema2 requires current bounded inventories. V1 rejection, not downgrade. Independent active camera/direct-input/worker protocol1 formats are different contracts, not old decoder schemas. |
| common camera/audio connection settings | ADB remains an explicit supported transport alongside experimental native USB. Missing usb_peer represents that current selection and passive saved settings, not an obsolete wire migration. T931/T977 tests pass. |
| platform input/output adapters | KScreen primary priority/boolean responses are supported OS API representations. The direct-input adapter includes the excluded Windows path. Preserve native ownership and backend boundaries; no platform code changed. |

The25 selected shared contract/storage/GPU/transport regressions pass alongside
all4 T267 tests. Raw logs and source hashes retained. Existing liveness capability
negotiation, unqualified-backend cadence and validated manual/fallback choices
have current functional safety purposes; they are not removed as compatibility
shims. This is a bounded caller/contract audit, not a claim that every source line
contains no future finding. Windows/EVDI implementations, blocked ADB ownership
T1090 and physical device acceptance remain outside this work. No new allocator,
resource lifetime, synchronization, admission bound or native cleanup path.
