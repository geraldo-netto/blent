# T873 asynchronous frame driver

The asynchronous frame driver preserves partial progress and deadlines across canceled waits. Completed-body authority is rechecked; dropping the transport still does not prove external helper retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The driver owns partial header/body offsets and absolute active-I/O deadlines,
so cancellation of an awaiting future does not restart or corrupt a frame. Reads
and writes progress independently; completed events alternate fairly. One fixed
64 KiB receive buffer bounds staging. Outbound slices are reborrowed from the
existing Multiplexer reservation after each nonblocking native call; no outbound
payload copy or per-frame task is introduced. Timers allocate lazily once per
direction and reset in place. These are implementation properties, not measured
throughput gains.

A complete receive event exposes its body until explicit consume. A complete
write event follows header/body and flush completion, then retires the mux
reservation. Any protocol/I/O failure makes the driver unusable. Stream ownership
can be transferred for externally bounded TLS shutdown. External helper/worker
retirement is still separate; dropping a stream does not prove it.

Thirteen permanent driver tests and 329 retained common tests pass offline.
All 37 scoped production functions (driver plus multiplexer) pass individual
80% LLVM coverage. Sixty-one production/test functions have cyclomatic complexity
at most 8. Tests cover real fragmented duplex I/O, cancelled waits, absolute
header/body/write/flush deadlines, idle reads, independent direction progress,
EOF/errors, invalid lengths, 256 stale generations, buffer identity and shutdown.

The initial draft exposed unfinished staging bytes and cached delivery permission
across partial-body revocation. The two permanent regressions in red.log failed
before their fixes and pass in tests.log. Delivery now rechecks the mux generation
and activation after the whole body; callers must still check the resource lease.

This is the independently testable I/O prerequisite extracted from T872. Resource
management, bounded consumer isolation, complete management-record publication
barriers (including inbound events during TLS flush), and actual helper retirement
remain T872. Broker composition remains T853. This driver does not establish
working production display or ADB-free support. No downloads or live restarts.

</details>
