# T1104: selected whole-operation TLS deadline

The TLS adapter previously fixed every handshake/protocol exchange at ten
seconds. The independent `connection::handshake::Timeout` now represents a
positive duration at most sixty seconds, default ten seconds. Its scalar is
private and immutable; invalid values cannot enter the adapter. It owns no
resource, borrowed view, callback, buffer or native state. Nanosecond precision is
retained by the contract; installed Tokio timer scheduling has millisecond
precision and does not guarantee an elapsed wall-clock latency bound.

Both TLS roles require the caller to supply that validated contract explicitly.
One original timeout covers TLS, exact peer proof and the whole authenticated
protocol preface. It does not restart after TLS completes. Local TLS configuration
construction precedes this asynchronous-operation timer, as before. All current
callers explicitly retain the ten-second default. Host user control and original
broker-link propagation are the separate T1105 prerequisite of T898.

## Permanent regressions

After declaring the required typed API and updating current callers, the original
fixed-duration implementation was left in place. The permanent selected-deadline
regression then failed with `T1104 selected TLS timeout ignored`. Only after this
actual behavioral failure was the operation changed to use its selected timeout.
The earlier caller-declaration compiler failure is retained separately and is not
claimed as red behavioral evidence. Calls inside Rust macros require explicit
updates as well as ordinary call expressions.

The same normal-suite test passes after the fix for both roles at 1, 37 and
60000 ms. An independently requested timer defines the installed runtime's
expiration boundary; the test never waits for production beyond that boundary.
Additional real stock-TLS tests admit a peer delayed eleven seconds under a
selected twenty-second budget in both roles. Other tests spend twenty milliseconds
reaching authenticated TLS, then keep the required preface incomplete: the
original selected thirty-seven-millisecond deadline still wins. A fresh deadline
after TLS would fail that independent timer oracle.

The portable contract checks its unchanged default, exact min/max, nanosecond
precision, zero, maximum-plus-one-nanosecond, maximal Duration, and 4096 seeded
values against an independent inclusive-range oracle. Existing malformed
certificate/key, peer mismatch, missing/wrong ALPN/preface, default deadline and
mutual-TLS application-data tests remain unchanged in their assertions.

Fresh offline native collection passes **1269 tests** (897 host, 370 common,
2 executable integration); three existing host opt-in benchmarks remain ignored.
All **25 scoped production functions** pass their individual 80% gate, minimum
95%. All 110 Rust function source estimates are at most 9; caller-update tool
helpers are at most 6. Twelve architecture checks pass. The updated TLS example
compiles. A broader examples check found the unrelated missing camera-probe
adapter import, recorded under T1106 with its own permanent failing compile test.
Its failure is retained separately and does not establish a TLS caller failure.

Automatic approval review initially timed out before any verification process
started; a single permitted retry of the smaller native command was approved.
No process was stopped or result truncated because of that review timeout. Installed rustc 1.98.1/cargo-llvm-cov 0.8.7
collect in a fresh exclusively owned target. Source estimates use cached pinned
Rust parsing, with native SonarQube still unavailable under T1019. No thresholds,
exclusions or existing tests are weakened.

## Ownership and evidence limits

Each asynchronous call owns its original stream while borrowing its original
identity and approved peer for the call's lifetime. TLS configurations retain the
T1102 original signing-key/store guard; selected signers independently retain the
same guard through synchronous signing and inner destruction. No new native
worker, pool, allocation admission, callback or stop implementation is introduced.

On rejection, partial TLS/preface failure, cancellation or expiry, owned futures
and streams are destroyed in their original scopes. These errors do not establish
native helper retirement. Accessory callers still retain independent native
completion/admission and identity consumers through their actual retirement.
Existing T1100/T1102 and broker shutdown regressions remain the evidence for those
fences. Logical completion, a timer or a reference count is not substituted for
native quiescence, settlement or safe reuse.

Deadline fixtures own bounded 8192-byte duplex channels, immutable certificates
within existing 4096-byte admission bounds, original futures and timers. Both
ends/futures remain contained in each case and drop on every exit, including
assertion failure; no detached task or native thread is started by these fixtures.
Conditional native cleanup still requires original work/destructors to return
and existing reapers/joins to finish. Tests establish hosted Linux/stock TLS
contracts, not unrestricted native safety, physical USB acceptance or unsupported
platform runtime support.
