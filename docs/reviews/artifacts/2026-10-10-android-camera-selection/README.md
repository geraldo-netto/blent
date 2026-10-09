# T800: explicit hardware encoder selection rejected

Four fresh-process runs on RugKing Pad 2 Pro SDK36 alternate default-first and
explicit-first. Stock MediaCodec encoder-by-type compared with an exact-format
inventory match requiring hardware acceleration. Both configure AVC Baseline,
Surface input, 30 FPS, 4 Mbps at 640x480, 1280x720 and 1920x1080. Fifteen synthetic
GL frames per format/policy; no camera sensor opened. No network or downloads.
Source/APK hashes in provenance.json; generator retained in scripts/benchmarks.

Every selection was `c2.unisoc.avc.encoder`, reported hardware accelerated. No
software fallback or encoding improvement found. Fresh first startup: default
169.1–175.7 ms; explicit 524.2–532.4 ms. First inventory traversal takes hundreds
of milliseconds; subsequent traversal takes about 0.9 ms. Smaller warmed startup
differences favor neither a different codec nor an improved execution path.

| Format | Default / explicit delivery median ms | Active PSS median KiB | Thread CPU median ms |
|---|---:|---:|---:|
| 640x480 | 26.51 / 26.80 | 38922 / 40649 | 105.56 / 402.46 |
| 1280x720 | 26.20 / 26.30 | 38832 / 38792 | 51.32 / 48.98 |
| 1920x1080 | 34.50 / 34.71 | 38912 / 38973 | 55.89 / 53.99 |

Delivery is synthetic submission-to-encoded-output, not camera/host presentation
age. Fifteen synchronous frames do not establish sustained capacity. CPU/PSS cover
the benchmark process, excluding vendor codec-server/GPU work. Tests alternate
order but do not eliminate warmup; raw data retained. Native acceptance is limited
to this device and tested formats. Pre-29 acceleration remains unknown.

Decision: retain stock selection/fallback. Do not add ranking, extra metadata
work, public selection contracts or user controls without an actual measured
benefit. Selection identity recorded by this diagnostic probe; no production
policy change or artificial selection-unit tests. Reconsider if a real device
selects software or a compatible alternate demonstrates lower delivery cost.
