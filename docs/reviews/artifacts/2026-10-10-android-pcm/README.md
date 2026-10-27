# T797 PCM staging and typed-write measurements

Bulk PCM staging and typed speaker writes preserved data and reduced selected replay costs. Silent native output does not establish acoustic latency or zero-copy delivery.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

RugKing Pad 2 Pro SDK36, exact fingerprint and APK/source hashes in results and
provenance. Baseline `b21fe37` already includes T795/T796. Candidate preserves bulk
little-endian conversion with per-reader scratch, uses typed nonblocking speaker
writes, reuses native timestamp scratch and writes/reads clock fields directly.
The final production change after measurement only adds sample-unit contract
comments; executable benchmark sources remain identified by provenance.

Use offline `scripts/benchmarks/audio-project.py`, then broadcast shell-only
AudioQueueReceiver with `--es probe pipeline` or `--es probe native-output` and a
unique run_id. Read private files/audio-queue.json through run-as. Both isolated
debug packages were compiled with `cmd package compile -m speed -f` successfully.
For each probe, fresh-process order was B1,C1,C2,B2,B3,C3. No downloads, microphone
capture or production-app replacement. Native output uses volume zero.

Pipeline probes use eight 500-call passes per run. Setup/prebuilt source bytes
are outside timing/allocation counters. CPU is thread-local; ART allocation
counters are process-wide and quantized. Medians of 24 passes:

| Per call | Baseline wall | Candidate wall | Baseline allocated | Candidate allocated |
|---|---:|---:|---:|---:|
| Packet decode | 168.112 µs | 139.464 µs | 4358.144 B | 2195.456 B |
| Portable output policy, accepting fake track | 63.639 µs | 54.585 µs | 65.536 B | 65.536 B |

Packet checksums match; permanent tests validate complete PCM, endian handling,
partial/zero/dead writes, truncation atomicity, cancellation and array ownership.

Silent native AudioTrack test submits 100 stereo blocks at 48 kHz through actual
AudioOutput/AndroidSpeakerTrack. Median total thread CPU: 500.659 ms baseline,
412.508 ms candidate. Elapsed 946.835 versus 901.818 ms; no physical latency
inference follows from these short scheduling-dependent runs. Both versions
report the same 3848-frame granted buffer, 44160 played frames and zero underruns
at completion; remaining queued silence is discarded during normal test cleanup.
Median process allocation is 262144 bytes for both. No end-to-end zero-copy claim:
owned PCM and Android/server copies remain. This experiment does not satisfy the
separate T718/T720 physical/acoustic acceptance work.

</details>
