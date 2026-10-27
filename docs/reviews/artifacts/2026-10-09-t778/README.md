# T778 runtime dispatch evidence

Runtime SIMD dispatch passed scalar-oracle and unsupported-build controls. Local conversion measurements do not establish native execution on other architectures or physical tablet performance.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The full normal capture suite passes its 80% per-function gate for 192/192
production functions. Forced auto/portable/AVX2 tests compare an independent
scalar oracle, odd/cropped dimensions, padded and unaligned sources, tiny spans,
split gaps, scales 1–4, worker dispatch and output guards. Invalid policies and
environment strings are rejected. A BLENT_NO_AVX2 build exercises unsupported
admission and generic fallback. GCC and Clang ASan/UBSan runs pass. This is local
x86 validation; a generic fallback build is not native execution on another ISA.

The replay is the same T768 workload at 1920×1080, scales 1/2, workers 1/4,
six damage patterns, five alternating before/after trials, 96 samples after 16
warmups. The final baseline was rebuilt from the exact 82f4b2a conversion source
and headers; its source bytes were checked against Git. No concurrent test or
compilation processes were started during these final trials. Every process
checks complete output against an independent scalar oracle before timing.
CPU measurements include damage marking and measurement-loop overhead.

`before.c` and `after.c` retain the exact historical replay sources and local
include paths, as in the T768 evidence. To reproduce elsewhere, replace only
those local includes with matching Git checkout paths and compile with `cc -O3
-pthread -I<matching host/evdi> replay.c -o replay`. Invoke `replay SCALE KIND
WORKERS 0`; KIND 0..5 is empty, distant, dense, skewed, narrow, fragmented.
These are historical evidence, not standalone maintained build targets.

Assembly checks confirm YMM instructions in both AVX2 kernels and none in the
portable kernel bodies. Portable release flags are unchanged. The auto threshold
avoids narrow work; the advanced environment policy permits local overrides.
The results concern conversion cost, not physical-tablet FPS or all hardware.

</details>
