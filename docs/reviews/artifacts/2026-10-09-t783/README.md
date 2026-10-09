# T783 native channel-order evidence

The normal-suite single-source-buffer regression failed on T782's implementation
before production changes (red.log). It now passes with mode/format changes,
resizing, cursor blend/restore/clipping, shared-slot retention and invalid format
options. Independent conversion oracles cover both channel orders, scales 1–4,
strides, spans, tails, output guards and portable/auto/forced SIMD. Invalid enum
values reject before touching null buffers. GCC and Clang ASan/UBSan pass.
The complete normal native gate passes all 207 production functions at ≥80%
executable-line coverage; no new exclusions. Changed-function complexity ≤9.

`paired.json` contains five alternating canonical/native trials, 360 processes,
1920×1080 RGBA input, scales 1/2/4, workers 1/4, dense/sparse/63-overlap damage,
with and without a 32×32 premultiplied cursor. Each process uses 16 warmups and
96 timed frames. Whole-output scalar checks run outside timing. Timed calls
include actual cursor restoration, normalization, damage, conversion and frame
publication. EVDI readback is mocked: this measures owned userspace work, not
GPU readback, compositor pacing or tablet FPS. No devices or network used.

The preserved replay includes the production test harness and checked argument
parser using historical absolute paths. Redirect includes to the matching
checkout and compile with `cc -O3 -ffunction-sections -fdata-sections
-Wl,--gc-sections -pthread replay.c -o replay`. Arguments are SCALE WORKERS
CANONICAL KIND CURSOR; kinds 0/1/2 are full/sparse/overlap. Parse the `BENCH `
stdout line. `run.py.txt` preserves the trial order.

Dense RGBA with cursor, median wall/CPU microseconds (canonical → native):

| Scale/workers | Wall | Process CPU |
|---|---:|---:|
| 1/1 | 760.178 → 479.230 | 783.452 → 490.977 |
| 1/4 | 610.800 → 144.490 | 1090.065 → 531.861 |
| 2/1 | 533.557 → 260.303 | 541.011 → 265.735 |
| 2/4 | 533.557 → 82.123 | 713.219 → 299.144 |
| 4/1 | 405.390 → 133.289 | 415.110 → 136.466 |
| 4/4 | 486.750 → 48.288 | 610.573 → 159.732 |

Sparse cursor cases improve by 0.43–0.69 µs in this replay. At 1920×1080,
native RGBA removes an 8,294,400-byte allocation and the full-copy case's
8,294,400-byte read plus write per frame; actual cache/memory-bus traffic is
not measured. Canonical mode remains explicitly selectable. Kernel code size
increases because channel order is specialized outside pixel loops.

`bgra-paired.json` reuses T782's retained replay against the T782/current binaries,
five paired trials, 60 processes. BGRA dense medians remain close: scale1/1worker
470.389 → 472.472 µs, scale2/1worker 257.065 → 256.445, scale4/1worker
130.528 → 133.715 (+2.4%). BGRA gets no removed-copy benefit; small layout/noise
differences are not claimed as improvements. This is one CPU, not universal
hardware acceptance.
