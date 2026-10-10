# T851 resource lease and settlement policy

The portable Rust/Kotlin state machines own four bounded resource slots and no
native handles. Tests cover consent/deadline checks, wrong order and duplicate
messages, stale connection/registration callbacks, independent cancellation,
actual-retirement barriers, reconnect and counter exhaustion. Rust includes 4,096
invalid callback mutations; Android includes 1,024 repeated bounded transactions.

Both languages compose the policy with the real multiplexer: cancelling camera
while Commit is in flight drains its late CREDIT without enabling media, waits
for actual channel retirement, admits generation two, and keeps display DATA
transferring. Never-installed attempts retain the untouched mux generation.
These are automated domain tests, not production USB/media acceptance.

Ten T851 Rust tests (including one internal exhaustion test) and ten Android
tests pass offline. The Rust validation also passes the 220-test common library
suite, five management tests and eight mux tests. Android additionally passes
management and mux tests. All 26 Rust and 23 Kotlin inventoried production
functions pass the per-function 80% gate. Supplementary native Kotlin counters
cover authored constructors/getters as well; all 54 methods reach 100% lines.
Complexity remains within nine. Source hashes, manifest, native counters, report
and test logs are retained alongside this file.

Production adapters must reserve bounded FIFO output before outgoing transitions,
honour whole-record Commit/Revoke completion barriers before dispatching peer
events, and report actual native/consumer/writer retirement. A timeout or kill
request cannot release a slot. The tablet's Result(Revoked) is exclusively a final
settlement acknowledgement and is sent only after its local retirement. These
adapter obligations remain in T853/T854; production integration is not claimed.
