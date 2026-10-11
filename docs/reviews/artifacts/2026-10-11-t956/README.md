# T956: camera probe selection boundary

Separate explicit hardware encoder selection from configure/render/drain/measure.
Preserve selection timing boundaries, regular-codec search, type/hardware/format
checks, default fallback, output deadline, byte/age counters and cleanup order.
Cached Kotlin 1.9.20 compiles exact probe against installed Android API 34;
all seven functions score <=9, selection 5 and run 6 (previous run 10).
Benchmark coverage exemption retained; no tablet launch or timing gain claimed.
