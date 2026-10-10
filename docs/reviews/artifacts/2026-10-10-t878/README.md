# T878 bounded local endpoint pump

Portable Endpoint contract exposes exact lease/channel identity, active authority,
allocation capacity, bounded producer offers and exact consumer completions.
Session adapts to this contract without exposing native sockets. Pump owns one
byte stream and one pending block per direction. It checks exact current identity
before every I/O poll; cancelled waits preserve offsets and ownership. Consumer
credit returns only after complete local write and flush. Active consumer output
and refused producer offers have independent absolute deadlines; idle reads do
not expire. Failure poisons only this pump, leaving resource retirement to its
owner. Native Completion remains separate from buffer/stream destruction.

Two initial-draft allocation bugs have permanent red-before-fix regressions:
a requested 64 KiB producer allocation could not enter a negotiated 4 KiB queue
even after a one-byte read, and Vec::resize's minimum growth could exceed a tiny
endpoint capacity. Producer allocations now use the smaller configured block and
endpoint capacity, including capacities below eight bytes. Fixed receive scratch
storage is reused. No throughput improvement is claimed without measurement.

Offline validation:81 broker tests pass, including11 new pump/adapter cases for
partial duplex I/O, retained credit, stalled flush/backpressure, idle/active
deadlines, stale/rebound leases, invalid extents, native errors and a real isolated
loopback TCP duplex/drop test. All32 scoped production functions pass per-function
80% LLVM coverage; reviewed complexity is at most9. Raw counters, source inventory,
hashes and red/green logs are retained. T853 still owns task scheduling, registered
endpoint connection, resource-wide teardown and actual native retirement checks.
No downloads, live services or USB devices were changed.
