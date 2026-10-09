# T791: coherent cursor-image cache

The optional GPU helper now fetches cursor pixels on invalidation instead of
every frame. Both cadence modes subscribe to shape notifications. Known changes
refresh directly; otherwise a synchronous pointer reply orders earlier events
before the bounded drain and cached draw. An unfinished drain forces an image
query. Existing damage processing stays with the event owner. An off-output
shape change retires its old Picture before publishing new shape metadata.

The first permanent regression fails on the original repeated image fetch.
An additional offscreen-change/re-entry regression caught and rejected an early
candidate with stale Picture/new metadata; it now passes. Shape changes,
hotspots, cropping, entry/exit, event floods, teardown/reopen, damage cadence and
existing metadata-transition regressions pass. Both GCC and Clang ASan/UBSan
native suites pass; all 55 GPU production functions meet individual 80% line
coverage. Changed C complexity is at most nine.

Five alternating trials of actual old/new `gpu_cursor` in isolated Xvfb, 1024
calls/run after warmup. Median run means, microseconds:

| Shape | Stable before | Stable after | Shape changes every frame: before | After |
|---|---:|---:|---:|---:|
| 16×16 | 11.653 | 11.099 | 27.028 | 27.494 |
| 32×32 | 13.901 | 11.445 | 32.779 | 33.892 |
| 64×64 | 20.370 | 12.156 | 49.866 | 50.625 |
| 128×128 | 60.967 | 14.305 | 130.566 | 134.695 |

Stable 64×64 cost falls approximately 40%. Request counts are unchanged: one
query plus composite per stable frame. Savings come from omitting the image
payload/allocation, not eliminating every X round trip. Shape churn pays roughly
1.5–3.4% overhead in this fixture for notifications/draining, without redundant
position requests for changes already queued. No desktop frame-rate, battery,
physical-GPU or complete-stream latency gain is claimed. The helper remains
opt-in; capture selection and cadence defaults are unchanged.

`raw.json` contains all trials. Reproduce offline by compiling `bench.c` twice
with `cc -O3`, once with the `host/gpu` sources from `c74047c` and once with this
commit. Include `scripts/benchmarks` and cached SDK headers; link `cursor.c`,
`events.c`, `cadence.c`, `options.c`, X11, Xfixes, Xrender and Xdamage. Run
`BLENT_C_REVIEW_ISOLATED=1 xvfb-run -a -s '-screen 0 256x256x24' bench SIZE CHURN`
for sizes 16/32/64/128 and churn 0/1. No live desktop or downloads are used.
