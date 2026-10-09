# T796 ART drift-cache replay

RugKing Pad 2 Pro SDK36. Baseline `ae0dd48`; exact APK/source hashes and fingerprint
in provenance/results. Use the offline `scripts/benchmarks/audio-project.py`
builder and shell-only `AudioQueueReceiver`, passing `--es probe drift` and a
unique `run_id` to the broadcast. Read private `files/audio-queue.json` with run-as.
Both isolated debug packages were compiled with `cmd package compile -m speed -f`
(success) before fresh-process B1,C1,C2,B2,B3,C3 trials. No microphone/speaker,
network transfer, downloads or production-app replacement.

Each run warms three 1000-call passes, then measures eight 5000-call passes using
unchanged completed native-rate windows. Every call still checks window age;
expiry and clear are checked after each pass. Median of 24 passes:

| Per correction | Baseline | Candidate |
|---|---:|---:|
| Wall time | 13.224 µs | 9.967 µs |
| Thread CPU | 13.093 µs | 9.881 µs |
| ART allocated bytes | 632.422 B | 16.384 B |

Every pass returns checksum 2,500,000. Process-wide ART allocation counters include
amortized setup; CPU counters measure the probe thread. At 100 calls/s, this replay
avoids approximately 61.6 kB/s of allocations and 0.32 ms/s of thread CPU. This is
not a live source/sink drift, release-build, acoustic or end-to-end latency claim.
