# T790: cursor/conversion fusion evaluation

Reject the general fused replacement; retain save/restore. The tested kernel
blends directly from an immutable desktop source while computing NV12, so it
removes both background copies and RGB overlay writes. Scale-specialized AVX2
code preserves blend saturation, box truncation and chroma rounding. It still
regresses the common scale-1/2 cases substantially.

Five alternating trials, 256 operations/run after warmup, one worker, 1920×1080
source. Median run means, microseconds; each process checks output against the
current overlay-plus-conversion implementation before timing:

| Cursor | Scale | Current | Fused |
|---|---:|---:|---:|
| 64×64 filled rectangle | 1 | 13.862 | 19.020 |
| 64×64 filled rectangle | 2 | 11.627 | 15.823 |
| 64×64 filled rectangle | 4 | 11.478 | 10.495 |
| 64×64 transparent arrow | 1 | 7.649 | 12.508 |
| 64×64 transparent arrow | 2 | 5.319 | 9.263 |
| 64×64 transparent arrow | 4 | 5.125 | 4.896 |
| 256×256 transparent arrow | 4 | 76.564 | 76.885 |

Initial all-semitransparent scalar trials overstated the scale-4 opportunity.
The final candidate includes the transparent-pixel fast path and AVX2 target;
the scale-4 saving is small, shape/size-dependent, and disappears for larger
images. The simplified candidate already assumes an aligned, contained cursor
and one conversion history. Production integration additionally needs clipped
edge blocks, BGRA/RGBA, old/new damage, unchanged pixels inside merged spans and
multiple stale output histories. No general improvement or safe admission rule
is established. Do not add that ownership complexity for these limited results.

The current copies are required to restore the desktop before stock EVDI grabs
and cursor changes. They must not simply be deleted. No production behavior or
existing permanent regression was changed. This rejects this measured design,
not every possible future fused/vectorized cursor kernel.

To reproduce offline, compile `candidate.c` using `cc -O3 -pthread
-DFUSED_AVX2`, with `host/evdi` and `scripts/benchmarks` include paths. Run on an
AVX2-capable machine: `candidate SCALE SIZE FUSED ARROW`, where scale is 1/2/4,
size is 16/32/64/128/256, and both flags are 0/1. Alternate current/fused runs for
five trials. `raw.json` retains all observations. No live display or downloads.
