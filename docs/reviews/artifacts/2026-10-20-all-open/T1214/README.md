# T1214: settle the original FIFO fixture on native startup failure

A FIFO fixture now retains and joins its original reader even when encoder startup fails. The unavailable-encoder regression passed; production encoder behavior and deadlines stayed unchanged.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The SDK base codec lacked libx264. Its actual reader exited before opening the
FIFO, leaving the test writer permanently blocked in open(wait_for_partner).
The retained diagnosis pins original process/thread/FIFO identities. Containment
waited for those exact processes before deleting only their original private FIFO
and directory; timestamps/CPU alone were not used as stall or retirement proof.

Permanent unavailable-encoder subprocess regression failed against the original
blocking fixture in5.01seconds (explicit5second error-response test contract),
then passed after repair. The final identical regression body runs its synchronous
fixture outside Tokio's async worker. All6 retained targeted cases pass: startup
failure, bounded partial/zero writes, WouldBlock, unwind joining, invalid native
names, native non-UTF8/space paths and distinct restart ACK identities.

The fixture opens its own nonblocking read/write descriptor before admission.
Every admitted original reader belongs to one guard; stop requests precede actual
join on normal completion, I/O/error propagation and unwind. Guard is destroyed
before the writer and private directory, so no thread can borrow released paths.
Partial writes remain bounded by the6144byte frame; zero progress rejects. Native
failure is propagated after actual join. No refcount or is_finished observation
stands in for that join; the latter only selects when to attempt settlement.
Cleanup assumes the existing native run observes its stop signal or finishes;
this change does not claim to make an uncooperative kernel thread killable.

All changes are cfg(test); production encoder behavior and EVDI adapters are
unchanged. Existing native FFmpeg positive controls remain. Local unchanged
Sonar-rule source model (cached tree-sitter0.25.2/ABI14 Rust grammar) measures all
changed/new functions1–7; native Sonar measurement remains T1019. Test fixtures
are exempt from production line-coverage thresholds; no test was removed or
weakened. No live device/service work, downloads or uploads.

</details>
