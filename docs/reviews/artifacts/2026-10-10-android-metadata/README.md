# T802 metadata query measurement

USB RugKing Pad 2 Pro, Android SDK 36; fingerprint recorded in each JSON.
Baseline: immutable commit `847b925`. Candidate: production source hashes and APK
hash in `candidate-provenance.json`. No dependency downloads or production app
replacement. Separate shell-only packages query metadata without media capture.

Reproduce offline with `scripts/benchmarks/capability-project.py --directory DIR
--package com.blent.capabilitybench.baseline --revision 847b925`, then build a
candidate without `--revision`. Install those isolated APKs locally. Broadcast
`com.blent.benchmark.CapabilityReportReceiver` with `run_id`; read private
`files/capabilities.json` through `adb shell run-as PACKAGE`.

Each format has one first query, five warmups and twelve batches of four reports.
Final comparison uses fresh processes: B6, C1, C2, B7, B8, C3. Force-stop only the
isolated package before each run. Median of 36 batch timings per format:

| Format | Baseline µs/report | Candidate µs/report | Reduction |
|---|---:|---:|---:|
| 640×480@30 | 20628.8 | 18924.2 | 8.3% |
| 1280×800@60 | 19988.2 | 15493.0 | 22.5% |
| 1920×1080@60 | 18799.0 | 12899.0 | 31.4% |
| 2560×1600@90 | 5151.7 | 2177.1 | 57.7% |

Full report payloads equal across all final runs. First-process query spans
445.89–460.28 ms baseline and 438.03–477.31 ms candidate: no established cold-start
speedup. Later formats' `cold_us` means first query for that format, not a fresh
process. Native metadata results remain request-scoped; no persistent cache.
These timings measure metadata reporting, not video FPS or application startup.

The production candidate uses bounded loops and tiny MIME arrays. The table
uses only the matched fresh-process runs listed above.
