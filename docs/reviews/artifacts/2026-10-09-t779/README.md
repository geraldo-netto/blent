# T779 evidence

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The permanent counted-copy regression fails before the fix and passes with
ASan/UBSan afterward. The full capture gate measures every production function.

`normalization.c` compares baseline 82f4b2a to this change using independent
processes in alternating order, five trials, 32 timed samples plus eight warmups.
Baseline includes the matching captured `capture.c` via BLENT_CAPTURE_SOURCE;
unused native paths are discarded by linker garbage collection. No devices.
Paired results were rerun after the coverage processes finished. Kinds: 0 sparse
64x64, 1 full frame, 2 63 identical full frames, 3 63 narrow fragments. Both builds
check channel order and padding. Timings are local conversion costs, not app FPS.
