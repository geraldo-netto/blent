# T1403 failed native input cleanup

The creating input worker remains the sole owner of its provisional/active
Session and native adapter. It installs that Session before configuration save,
so a rejected or unwinding save cannot discard the new native resource. Cutoff
revokes authority immediately. Failed commands return/cancel their async receipt
before cleanup retries; no successor native command can overtake those retries.

Retirement sets cleanup_pending and retries the same original adapter on the
creating thread. A failure never clears Session ownership. Only accepted native
release permits its destruction and subsequent replacement. Every retained worker
continues to consume its original bounded pool admission; no extra cleanup thread
or unbounded error history is created. The retry wait is a 100 ms observation
interval, not a deadline, and only failed cleanup causes periodic wakes. One
warning is emitted per cleanup episode; status exposes pending debt.

Successful settlement and exact original thread/TLS exit are separate necessary
conditions for Completion. A deliberate thread-exit-without-settlement control
cannot release admission. Adapter command/retirement panics retain the Session
outside unwinding; unhandled outer/destructor failure does not set settlement.
No unconditional leak-freedom claim is made: eventual release requires returning
native operations, eventual release acceptance and worker/observer scheduling.
Native Windows behavior remains T1409. Monitor/normal process-exit integration is
separately recorded as T1410; this worker fix alone does not certify daemon exit.

Independent adapter tests reproduce three premature-owner-drop paths before the
fix and a replacement that made no cleanup progress. They then verify successful
button release before destruction, preservation of a separately held physical
button, caller loss, failed save, failed event, replacement and adapter unwinding.
The final instrumented input selection passes 116 tests. Changed production
functions meet >=80% executable-line coverage and complexity <=9 (metrics JSON).
Windows library, application and tests cross-compile; they were not executed.
