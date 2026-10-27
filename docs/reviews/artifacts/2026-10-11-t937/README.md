# T937 camera local ownership and independent cleanup

Camera ownership tracks route exposure separately from actual endpoint and capture-consumer cleanup. Either cleanup order is supported; socket EOF or Stop cannot release the original claim.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Focused lifecycle, projection, control and original-resource authority contracts
separate the camera owner from display and hardware capture. Permission-pending
offers contain no credential or endpoint and open no native factory. Choices and
permission callbacks match the exact retained lease/prompt and original runtime;
pending authority expiry is checked before late callbacks even before the protocol
timer processes closure. Preparation and capture admission remain independent.

Publishing a camera route records possible external consumer exposure before
returning credentials, even if delivery fails. Closing retains both actual local
endpoint/pump retirement and an independent capture-consumer completion claim.
Only exact retained consumer cleanup is accepted after logical connection stop;
stop requests, timeouts and TCP EOF do not establish capture-worker completion.
Either cleanup order is supported. A stuck consumer quarantines camera while
an independent display continues. Uncertain native-proof publication is retained
without replay or slot reuse.

The permanent T937 pending-stop regression fails before protocol cancellation
is ordered ahead of native retirement, then passes unchanged. Additional tests
cover stale replies and cleanup, denial, provider failure, both retirement orders,
local TCP camera greeting/feedback, permission expiry, and bounded invalid clocks,
metadata and lifecycle inputs. The loopback consumer proof explicitly means no
hardware worker was started; it is not physical camera or codec evidence.

All 662 offline connection tests pass. All 124 methods in the six scoped production
files meet 80% individual native JaCoCo executable-line coverage; all eleven authored
constructors meet 100%, supplementing T848. All 164 reviewed production/test
functions have cyclomatic complexity at most nine. Inventory, exact counters,
source hashes and red/green logs are retained.

Production camera capability remains disabled. T936 composes private IPC and
original worker lifetime; T932 must obtain actual isolated hardware-worker retirement
proof and wire Activity capture before enabling capabilities. Joined physical
acceptance remains T858 and the host adapter T933. No downloads, installation,
reloads or tablet trial occurred.

</details>
