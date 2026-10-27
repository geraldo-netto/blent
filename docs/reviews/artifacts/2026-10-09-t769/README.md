# T769 implementation evidence

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

`t769-red.log.gz` records the permanent regression failing on the original
row-count scheduling. `t769-final.log.gz` records the same test passing after
weighted scheduling. `coverage.json` covers every production capture function.

`t769-paired.json.gz` retains five alternating-order before/after timing trials.
`before.c` and `after.c` are exact historical replay sources with the original
local include paths, not maintained standalone executables. Their included
production code is the `a3c053e` baseline and the source-hashed implementation
in `summary.json`, respectively. Each process validates against an independent
full-output scalar oracle before timing. These files record an experiment;
changing their includes to a later source revision does not reproduce it.

No display, encoder or tablet runs in this replay. It demonstrates a scheduling
latency/CPU tradeoff, not end-to-end throughput or device acceptance.
