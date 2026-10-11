# T952: cohesive archived cursor fixture setup

Extract cursor shape creation from fixture allocation/setup. Preserve all pixels,
branch behavior, loop bounds and original baseline/fused comparison assertions.
12 local native runs cover scale 1/2/4, baseline/fused and solid/arrow cursor.
All ten candidate functions now score <=9; original init score was 10.
All 19 parser fixtures still pass and inventory every original function plus the
new shape helper. Benchmark code retains its explicit coverage exemption.

The archived candidate source was structurally refactored on 2026-10-11;
2026-10-09 timings remain historical observations of the earlier source, available
in git history. These new parity runs establish no speedup or device acceptance.
