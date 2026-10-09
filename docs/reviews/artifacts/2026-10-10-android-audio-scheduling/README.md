# T801: native audio scheduling

Offline builds, cached dependencies, USB transfer to RugKing Pad 2 Pro SDK36.
Isolated benchmark packages; production app untouched. Audio output muted.

`adjusted/modes-1..4.json` alternates standard/low-latency order. Each mode writes
48000 stereo frames, requesting identical capacity. All runs granted the requested
mode, reported 3848 buffer frames, and had zero underruns. Timestamp-based queue
estimate corrects outstanding frames for timestamp age. Standard estimates were
174.8–176.8 ms; low latency 107.4–109.2 ms. This is a native pipeline proxy, not
physical acoustic latency. No smaller buffer or hardware offload claim.

CPU ranges overlap (standard 60.8–77.0 ms, low 65.5–81.1 ms). Initial `modes-*.json`
trials always ran standard first and exaggerated CPU improvement through warmup;
retained for audit, superseded by alternating trials. No CPU benefit established.
Zero-progress writes still occur about 382–418 times per second. Retain 2 ms
retry and per-attempt health checks: no measured safe adaptive pacing benefit.

Production requests low latency only with a persisted speaker-only opt-in,
default false. `production-low.json` confirms actual adapter request/reporting:
low latency granted, unchanged buffer, zero underruns. The normal fallback remains
available; Android chooses actual mode. Provenance JSON records source hashes.

Permanent T801 focus regression failed before change with 10 timestamp calls in
50 ms while paused, passes with zero. Rendering now awaits focus events; local
socket Stop, reader completion, cancellation, terminal focus loss and late gain
remain covered. Normal 250 ms write budgets and route checks remain unchanged.
No AAudio/JNI backend or new native dependency was justified by these measurements.
Physical duplex/route/acoustic acceptance remains tracked separately by T718/T720.
