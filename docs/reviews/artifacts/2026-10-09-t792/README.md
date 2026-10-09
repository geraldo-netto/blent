# T792: vectored GPU packet output evaluation

Retain current output. Reject this replacement because the 64 KiB packet case
regressed in every paired trial despite reducing writer CPU and syscall count.
No production changes or new zero-copy claim.

Five alternating trials, 256 packets/run through an anonymous pipe to a draining
child. Both variants retain the exact framecrc header and Adler checksum work.
Median run means, microseconds per packet:

| Payload | stdio wall | writev wall | stdio writer CPU | writev writer CPU |
|---|---:|---:|---:|---:|
| 64 B | 0.554 | 0.544 | 0.551 | 0.541 |
| 2 KiB | 1.364 | 1.302 | 1.259 | 1.209 |
| 64 KiB | 21.488 | 25.407 | 21.308 | 16.106 |
| 1 MiB | 395.698 | 381.199 | 257.968 | 249.929 |

The 64 KiB latency regression is approximately 18%, while writer CPU falls
approximately 24%. Separate `strace` runs confirm three `write` calls per
baseline packet versus one `writev`; the extra final write is benchmark JSON.
This demonstrates that fewer syscalls need not improve pipe delivery latency.
It does not establish an application latency difference or a universal kernel
behavior. The probe validates total transmitted length; it is not a replacement
for permanent byte-equivalence and partial/error-write regressions if this
candidate is reconsidered. No production callback lifetime was changed.

The optimization concerns encoded packet buffering, not raw frames, and remains
lower impact than capture/conversion work. Do not introduce a size-dependent
dispatch from one synthetic pipe workload alone.

Reproduce offline: build `packet.c` with `cc -O3 -Wall -Wextra -Werror`, include
`host/gpu`, `scripts/benchmarks` and the installed/cached SDK headers, and link
`libavutil`. Run `packet BYTES 0` for current output or `packet BYTES 1` for the
candidate, with payloads 64, 2048, 65536 and 1048576. Alternate order for five
trials. Trace separately using `strace -c -e trace=write,writev`. No GPU, live
desktop or external dependency download is used.
