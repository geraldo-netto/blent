# T805: release encoded storage before decoder-input ACK

The camera sender releases encoded storage after transport flush, before waiting for the acknowledgement. Delayed-ACK regressions verify shorter lease holding without adding packet overlap or changing ACK-limited throughput.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Same permanent delayed-ACK fixture, Robolectric adapters, real production
CameraCapture/CameraLink/CameraWire. Baseline source from b1009d0; candidate releases
a single-owner buffer lease immediately after full transport flush. Source ranges
and packet bytes are checked. Fake codec; these are synthetic sender hold times,
not native camera FPS or host presentation measurements.

| Injected ACK wait ms | Before hold µs | After hold µs | After full cycle µs |
|---:|---:|---:|---:|
| 0 | 59 | 30.5 | 44.5 |
| 5 | 5088 | 20 | 5091 |
| 20 | 20126 | 44.5 | 20185 |
| 50 | 50178 | 77 | 50216.5 |
| 100 | 100194 | 92 | 100254 |

Baseline exact-method filtering runs SDK34 (three samples per delay); the full
candidate suite runs SDK27/34 (six samples). Raw samples remain in measurements.json.
Candidate ran under JaCoCo, baseline without it; absolute small overhead is not a
microbenchmark claim. The causal benefit is removing ACK wait from the lease.
No additional frame storage, asynchronous worker or packet overlap. ACK-limited
throughput remains unchanged; reject extra overlap without a measured benefit.

Regression first failed: buffer still owned when ACK read begins. It now passes
for delayed, invalid/reordered and truncated ACKs; native release failure is
attempted once. Transport flush failure never invokes ownership-transfer callback.
Missing-ACK timeout and socket Stop retire the connection after release. Existing
freshness/keyframe, bitrate, startup and cancellation tests remain. Callback runs
after flush even when the subsequent ACK fails; finally handles earlier failures
and empty/dropped packets. This preserves one packet in flight and bounded staging.

</details>
