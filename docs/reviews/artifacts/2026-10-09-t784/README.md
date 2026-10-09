# T784 — configuration write coalescing

Configuration packets now reuse the frame writer's vectored header/payload loop.
No allocation, payload copy, new queue, batching delay or wire change. Frame
packets retain their existing header and sequence representation. Zero writes
fail with WriteZero; transport errors, including Interrupted, propagate as before.
The enclosing one-second timeout still bounds either kind of packet.

Five counterbalanced local TCP trials, 3,000 packets per variant and size,
TCP_NODELAY enabled, receiver drained after each packet. The benchmark executes
the production writer and the former three-write sequence, backed by actual
blocking TCP write/writev calls (one syscall attempt per counted call in these
successful runs). No USB/tablet or end-to-end video latency claim.

| Payload | Former median | Coalesced median | Writes per packet |
| --- | ---: | ---: | --- |
| 64 bytes | 9.489 us | 4.699 us | 3 → 1 |
| 512 bytes | 9.649 us | 4.880 us | 3 → 1 |
| 4096 bytes | 9.749 us | 4.980 us | 3 → 1 |

Times are medians of five per-trial p50 send-through-receive times. Raw p95 and
counts are retained in `benchmark.json`. Configuration is infrequent, so these
savings should not be advertised as a per-video-frame improvement.

Permanent normal-suite regressions cover empty packets, 4,550 bounded length /
short-write combinations, byte-exact output, scalar-only writers, WriteZero,
Interrupted and BrokenPipe. `red.log` records the one-write regression failing
before the change. `green.log`: 26 stream tests pass; opt-in timing benchmark is
separate. LLVM executable-line counters: all three changed production functions
100% (`coverage.json`, raw `coverage.lcov.gz`). Existing stream timeout and frame
partial-write regressions pass. Changed functions remain at cyclomatic <=9.

Reproduce without downloads:

```
cargo test --offline -p blent --lib stream::
cargo test --offline -p blent --release --lib t784_tcp -- --ignored --nocapture
CARGO_LLVM_COV_TARGET_DIR=/fresh/target cargo llvm-cov --offline -p blent --lib --lcov --output-path coverage.lcov -- stream::
```
