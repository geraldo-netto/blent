# T788: register-fused box conversion evaluation

Reject the tested fusion; retain the production 1 KB tile. No production
behavior or dispatch changes. This is a result for this candidate, not proof
that every possible fused kernel must regress.

The candidate reduces one 2×2 output block at a time, immediately applying the
existing, inlined color arithmetic to its 16 reduced bytes. It preserves the
per-pixel truncation before chroma averaging. The smaller working set sacrifices
the color kernel's wider vectorization and instruction amortization.

Five alternating baseline/candidate trials on the same host, 96 samples/run;
median of run medians in microseconds, 1920×1080 source:

| Workload | Workers | Scale | Tile | Fused |
|---|---:|---:|---:|---:|
| Full frame | 1 | 2 | 258.899 | 1036.306 |
| Full frame | 4 | 2 | 77.192 | 277.855 |
| Full frame | 1 | 4 | 133.923 | 330.650 |
| Full frame | 4 | 4 | 45.339 | 95.457 |
| Distant damage | 1 | 2 | 3.638 | 4.740 |
| Narrow strip | 1 | 4 | 10.871 | 13.036 |

All twelve tested combinations regress. Each timed process first checks the
independent scalar oracle. The unchanged permanent `scaled-simd` suite also
passes against the candidate under ASan/UBSan, including BGRA/RGBA, tails,
padding, gaps, guard bytes and portable/auto/forced paths. These are isolated
conversion measurements, not application latency or tablet FPS.

`raw.json` retains every observation; `metadata.json` records the baseline and
source hashes. `bench.c` is the archived T782 replay adapted to current types.
To reproduce offline from this revision, copy `host/evdi/*.[ch]` into two private
directories. In the candidate directory replace `conversion_box.h` with
`candidate.h`. Compile `bench.c` twice with `cc -O3 -pthread`, putting each
directory first on the include path, followed by `scripts/benchmarks`. Run
`bench SCALE KIND WORKERS 0` for scales 2/4, kinds 1/2/4 and workers 1/4,
alternating order across five trials. No live capture, dependency modification
or downloads are required.
