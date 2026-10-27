# T901 — bounded Linux broker presentation client

The Linux presentation client keeps bounded transactions and native worker ownership outside redraw. Stale commands cannot starve refresh, and closing the client does not stop independent broker/media owners.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The independent `control_client::Client` port exposes immutable status, bounded
submission, logical close and actual retirement without OS or GUI types. Queuing
an action grants nothing: the Linux worker revalidates its T900 broker lifetime,
displayed identity and ticket immediately before sending. Protocol rejection is
visible and invalidates the old view; no approval is replayed on replacement.

The Linux factory shares one authenticated on-demand Connector and one retirement
admission. One retained native thread owns a Tokio runtime, serial packet socket
transactions and the presentation model. Complete matching-request snapshots are
assembled before publication. Malformed direction/order, unexpected descriptors,
wrong request IDs and incomplete/overdue snapshots fail closed. User-selected
limits have validated defaults: startup 10 s, transaction 5 s, polling 500 ms,
queue 8 (maximum 32); list bounds match the broker's supported maxima. Whole
transactions have one absolute deadline. Status diagnostics are capped at 512
characters. No file paths or descriptors enter the portable presentation port.

Close wakes the retained worker without waiting on native I/O. Its runtime can
still wait for outstanding native starter work, so terminal status is never proof
of retirement. Existing retirement_work::Thread observes actual join without
blocking the shared reaper. Independent broker/media lifetimes are not owned by
the GUI client and are not implicitly stopped by closing its control socket.

The draft stale-command refresh regression failed before its fix: each stale UI
command renewed the polling sleep. A fixed absolute refresh deadline now wins
under continuous stale traffic. Other permanent tests cover bounded queue/close,
peer loss, startup failure, malformed replies/descriptor injection, durable
rejection, same-counter reconnect, native retirement admission and panic while
native runtime cleanup remains blocked. The real broker runs against an isolated
synthetic USB inventory. A timed child process exhausts its own descriptor limit
to exercise real runtime-creation failure, restores it, and exits; host limits
are never changed.

69 native Linux broker tests pass, none ignored. All 22 new production functions
meet 80% individual executable-line coverage, including runtime-creation failure.
All 49 reviewed production/test functions meet complexity at most 9. Native LCOV,
source inventory/hashes, red/green logs and gate results are retained. The reused
LLVM target is scoped to these newly introduced modules; no broad coverage claim
or stale generic mapping from unrelated sources is used for acceptance.

GUI widgets remain T894, physical pairing T855 and media integration T857–T860.
Windows native broker/accessory parity is unimplemented and tracked under T902;
portable interfaces are not support. No live USB access, installation or download
occurred. Packaging remains deferred under T861.

</details>
