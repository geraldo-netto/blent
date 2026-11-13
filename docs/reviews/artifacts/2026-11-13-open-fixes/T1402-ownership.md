# T1402 asynchronous input ownership

A backend reserves one bounded retirement admission before spawning its original
native thread. Only that thread creates/uses native input. Short local mutex
sections publish status/authority and admit work; none executes native work or
waits for a response. Replies use asynchronous oneshots and authority watches.

One backend admits 17 outstanding commands, with at most 16 queued. Both original
native work and its receipt retain the same admission permit; neither completed
unconsumed receipts nor cancelled but unfinished calls bypass the bound. Dropping
an unpolled receipt revokes only its exact original authority and wakes idle
cleanup. Backend loss closes admission even when receipt senders survive it.
Only one native operation executes per backend, so successor authority waits for
the original operation. Cancelled queued work is discarded before native effects.

The session supervisor records the input worker's original Completion before
listener startup. Its native retirement fence therefore includes input worker
exit, including on cancellation and caller loss; session reuse cannot rely on
logical controller stop alone. Caller cancellation never joins on the executor.
The retirement pool joins the original thread before releasing its admission.
Cleanup liveness assumes native calls eventually return and the observer runs;
a stalled original worker stays funded. Failed native release remains T1403.

Socket read-ahead is bounded to 16 existing 64-KiB-limited frames. Buffered frames
are dispatched in order; Close, read failure, ownership loss or overflow drops
the pending reply and discards buffered input. There is no spawned reader/waiter.
Pure dispatch/wire conversion borrows only during synchronous admission; reply
futures own their authority, permit and response channel, not borrowed UI state.

Permanent red controls reproduce executor blockage and session retirement before
its input worker. The final instrumented input selection passes 110 tests; a
separate session-related selection passes 94 tests (overlap is not added into a
unique-case count). All 42 measured production functions have >=80% native line
coverage; changed-file complexity has no score over 9. Counters come from the
exact executed executable, without stale unrelated cached objects. Native Windows
code/tests cross-compile; T1408 retains actual Windows acceptance/counter work.
