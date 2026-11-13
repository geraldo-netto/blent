# T1410 native input/session/endpoint settlement

The original regression proves that Monitor::retire discarded Runtime::stop's
pending Completion and immediately made its serial/slot available. The monitor
now installs one retirement record per admitted slot before the first await.
Records retain the original session proof, routes and physical identity. A pending
record excludes its slot/serial/proven aliases, while independent slots continue.
Cancellation leaves the record installed. Completed records transfer remaining
route cleanup to the existing pending-route owner; they never infer native exit
from an empty inventory or elapsed time. Normal shutdown drains those proofs.

The original route owner also receives the same session proof before creating
native route resources. Its bounded retirement record holds native device/local
port leases through both route cleanup/thread exit and session completion. Losing
the whole monitor cannot drop that lease early. Connection cutoff may reject
route retirement while the session is still live; the monitor stops that session
before waiting for route retirement. The session proof depends on input/capture
workers, not routes, so this order has no circular cleanup dependency.

The direct-input pool now closes admission under the same mutex used to reserve
workers. A process-entry guard runs after producers and their runtime are gone,
then waits for that pool to drain. Original workers remain funded until successful
native cleanup and actual native thread/TLS exit. No additional native owner is
spawned during shutdown and no fixed elapsed deadline retires debt. Forced OS
termination cannot execute this guard. Existing Windows display-route shutdown
failure/native acceptance remains explicitly blocked in T1352.

Regression controls cover early reuse, cancellation before stop's first await,
sibling progress, bounded per-slot records, alias exclusion, recovery, monitor /
connection loss, duplicate/late lifetime binding, route-thread exit without input
settlement, pool capacity and closed admission. A child-process test exercises the
actual input ProcessExit guard without closing the parent test runner's pool.
The updated session fixture uses a fresh attachment after stopping the old runtime:
terminal authentication retirement is preserved, never weakened to allow reuse.

The final instrumented input/USB run executes 177 tests, all passing. All 28 measured
shared production functions have >=80% executable-line coverage; changed-file
complexity has no score above 9. The exact executed instrumented executable supplies
the retained counters. Linux accessory-enabled application/library checks pass;
Windows default application/library/tests cross-compile. Native Windows entry,
lease, global-button and per-function counter acceptance remains T1408/T1409;
cross-compilation and Linux adapter execution are not substitutes.
