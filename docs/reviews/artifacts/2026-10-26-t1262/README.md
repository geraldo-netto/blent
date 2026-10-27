# T1262: require the current initial stream format

Initial Android stream greetings require codec and both encoded dimensions. Partial updates remain valid after initialization; hosted full-suite tests passed without inventing panel-based formats.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Android requires codec and both encoded dimensions in the initial greeting.
Native panel hints and invented 1920x1080 dimensions no longer authorize a decoder
format. Established formats still supply omitted codec/dimension/FPS fields for
partial updates. Removed the now-redundant encodedDimensionsKnown state and the
unused panel arguments; updated current callers and wire documentation together.

The permanent seven-subset missing-field regression fails on both hosted API 27
and 34 before the change, then passes. Existing numeric/range/seeded mutation,
resize/reconnect and stale-greeting coverage remains. Old compatibility success
expectations now require rejection; the resize/reconnect test additionally accepts
a current complete greeting and rejects the old socket's later update. The
existing frame-rate integration fixture now sends a complete initial greeting
before its partial updates, retaining its 30/90/invalid-FPS assertions.

The full offline Android suite passes 2710 tests, including the concurrent T1263
fix in the same verification workspace. caller-update.log.gz retains the prior
full run's two failures caused by the obsolete test greeting, before that caller
was updated. No tests were skipped or timeouts relaxed. JaCoCo 0.8.15 native JVM
method counters show 100% for read, dimensions, the three changed ControlSession
methods and the inventoried constructors. The local Kotlin PSI 1.9.20 complexity
check records maximum 8 across changed production/test files. Raw report and
source hashes are retained. Hosted tests do not establish physical Android support.

StreamFormat is stateless: it borrows one bounded control JSON document and the
previous validated immutable format for a synchronous call, then returns a new
immutable format or rejection. No native allocations, retained callbacks or
ownership transfers were introduced; connection reset still clears the one
current-format owner before a replacement greeting.

</details>
