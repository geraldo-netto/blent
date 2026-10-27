# T795 native ART queue replay

An isolated Android queue replay reduced CPU time and allocation while preserving PCM checksums. Synthetic transfer ratios do not establish physical audio latency or source/sink behavior.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

RugKing Pad 2 Pro, SDK36, exact fingerprint in JSON. Baseline `da4938f`; candidate
source/APK hashes in provenance JSON. Isolated debuggable benchmark APKs only,
installed over USB. No microphone, speaker, network transfer or native playback
is used. These are ART CPU/allocation measurements, not end-to-end audio latency.

Build offline with `scripts/benchmarks/audio-project.py --directory DIR --package
com.blent.audiobench.baseline --revision da4938f`; build candidate without revision.
Broadcast `com.blent.benchmark.AudioQueueReceiver` with `run_id`, then read private
`files/audio-queue.json` through `adb shell run-as PACKAGE`.

Root trials are fresh-process runs in order B1,C1,C2,B2,B3,C3. `compiled/` repeats
that order after `adb shell cmd package compile -m speed -f PACKAGE` succeeded for
both isolated packages. No claim about release-build compilation is made. Each
run warms three 1000-block passes, then measures eight 2000-block passes. Inputs
are prebuilt immutable PCM; queue/setup allocations are amortized into results.
Runtime allocation counters are process-wide, CPU counters cover the probe thread.

Compiled-trial medians across 24 passes:

| Metric per aligned identity block | Baseline | Candidate |
|---|---:|---:|
| Wall time | 194.163 µs | 55.435 µs |
| Thread CPU | 192.361 µs | 54.737 µs |
| ART allocated bytes | 2211.840 B | 98.304 B |

This removes about 211 kB/s of allocations at 100 aligned blocks/s in this replay.
All identity checksums and five signed-rate PCM/time/discontinuity checksums match.
A separate 1200-tick native-clock replay records actual array identity: zero-ppm
transfers 1197/1197 outputs; each nonzero ±500/±1000 ppm case transfers 197/1197
while warming native windows, then retains interpolation. These synthetic ratios
are not the unknown frequency in a physical source/sink session. T718/T720 native
acoustic/clock acceptance remains separate. No SIMD/ring benefit is established
for the remaining interpolation path by this experiment.

</details>
