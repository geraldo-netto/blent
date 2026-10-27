# T782 evidence

Permanent independent-oracle tests were added before the optimization, passed
on the portable implementation, and pass on the new auto/forced AVX2 paths and
BLENT_NO_AVX2 fallback. This is an optimization, not a newly found pixel bug.
Tests cover scales 2/3/4, cropped/unaligned/padded inputs, 2–514 pixel output
widths, chunk boundaries, tiny and large split spans, workers and output guards.
GCC and Clang ASan/UBSan pass; the full gate passes 198/198 production functions.

Five counterbalanced before/after trials cover scales 1/2/4, workers 1/4 and all
six historical T768 workloads. Replays use 1920x1080 padded input and check the
full output against an independent scalar oracle. CPU includes marking/timing.
The exact replay sources retain historical absolute include paths; redirect them
to matching baseline/current checkouts and compile with `cc -O3 -pthread
-I<host/evdi> replay.c -o replay`. Arguments: SCALE KIND WORKERS 0. No devices.

Worker-local scratch is 1 KB. No syscalls occur per tile.
No full-frame temporary, heap allocation, global ISA flags or downloads.
