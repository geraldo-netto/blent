# T768 implementation evidence

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The permanent regression checks that distant rectangles do not overwrite the
unchanged gap, while fragmented updates use a conservative contiguous span.
`t768-red.log.gz` records the original failure; `t768-fast-green.log.gz` records
the complete conversion suite passing after adaptive and contiguous kernels were
separated. Additional permanent cases cover randomized unions and threaded spans.

`t768-kernels-paired.json.gz` contains five alternating-order trials over 240
single-session processes. Each process checks a full-output independent oracle
before timing. `summary.json` identifies the source bytes, representation costs
and limitations. Baseline is `05d8577` (weighted scheduling, before adaptive spans).

The two `.c` files are exact historical replays with their original local include
locations, not standalone maintained executables. Their include sources must be
restored from the baseline or the recorded source hashes to repeat this result;
including a later revision changes the experiment. No live EVDI/GPU/tablet is used.
