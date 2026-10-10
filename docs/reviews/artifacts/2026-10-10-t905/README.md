# T905 — bounded Android local-channel byte pump

ConnectionLocalPump binds exactly one resource lease and one of its media
channels to the existing prompt ConnectionIo boundary. The serialized owner
performs no socket operation or wait. One pending producer block and one exact
consumer ring loan bound each direction. Full producer blocks transfer ownership;
partial reads copy only their returned extent. Incoming ring bytes pass directly
to native writes without a staging copy. Receive credit returns only after the
entire exact slice and flush complete. This is a structural copy reduction, not
a measured throughput/latency gain.

The existing fixed StreamConnectionIo workers isolate native read, write and
close. One blocked direction cannot block the other; a failed pump never stops
siblings or releases a resource lease. Resource owners must retire every channel
and listener before calling nativeRetired. Idle reads have no data deadline;
backpressure and active writes/flushes have absolute budgets. Both budgets are
checked before any new transfer. That last boundary has a permanent regression
which failed before its fix: fresh input was forwarded at the opposite direction's
deadline. Cancellation suppresses late completion and retains borrowed bytes.
Actual native exit is required for retirement; failed close remains quarantined.

The default block is 65,536 bytes, configurable down to one. Composition must
cap it to the negotiated queue capacity; this pump does not infer OS endpoints
or remote authority. Limits reuse validated ConnectionRunLimits. Listener/setup
ownership is T906, Android IPC/activity binding T907, host integration T908 and
real display/touch acceptance T857. Production capability advertisement remains
disabled until that composition is ready.

570 connection tests pass, none skipped. New tests cover exact ring loans,
partial progress, queue backpressure, idle/deadline distinction, malformed and
foreign/duplicate callbacks, revoked leases, close/admission failures, counter
and range boundaries, 512 seeded exact-transfer cases, sibling isolation and
real local socket duplex with confirmed I/O-thread retirement. All 16 indexed
functions and all 20 executable JVM methods (including authored constructors)
meet 80% individual line coverage. All 45 production/test functions have
complexity at most 9. Native counters, hashes, red/green logs and test results
are retained. The loopback test ran on the host JVM; Android device validation
remains explicit in T906/T857. No downloads or live USB changes occurred.
