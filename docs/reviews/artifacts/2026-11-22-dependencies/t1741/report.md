# T1741: input WebSocket read-buffer sweep

8 KiB is the closest measured compromise. It retains nearly identical interactive
latency and gains about 5–8% on 64-KiB bursts, but does not dominate 4 KiB: the final
small-frame burst confirmation suggests about 5% lower throughput, with uncertainty.
For the input server's predominantly small messages, retaining 4 KiB is the
conservative default. Choose 8 KiB only with that tradeoff explicit; these data do
not justify a claim that a larger buffer universally improves input performance.
No production default, protocol limit, repository file, installed process or device
was changed by this investigation.

## Method and scope

Built the existing example with:

```
cargo build --offline --locked --release -p blent --example websocket_buffers
python3 /tmp/blent-buffer-sweep-20261122/run.py
python3 /tmp/blent-buffer-sweep-20261122/confirm.py
python3 /tmp/blent-buffer-sweep-20261122/confirm_small.py
```

Each recorded command is a fresh process:

```
taskset -c 2,4 target/release/examples/websocket_buffers --buffer-kib SIZE --payload 200 --clients 8 --frames 10000
# Small bursts: same, plus --burst.
# Large bursts: --payload 65536 --clients 4 --frames 1024 --burst.
```

- Initial: 18 balanced randomized blocks, 4/8/16/32/64/128 KiB, three workloads;
  324 measured runs plus 18 excluded warmups. Each size occupies every ordering
  position three times; workload order rotates. Seed 17380022.
- Independent finer sweep: 12 blocks, 4/6/8/12 KiB, same workloads;
  144 measured runs plus 12 excluded warmups. Seed 17380024.
- Final planned confirmation: 60 pairs at 4/8 KiB for small bursts; 120 runs,
  exactly 30 pairs in each ordering. Seed 17380025. No additional sampling after it.
- Total: 588 measured processes, all successfully completed; 30 warmup processes.
  Every echoed payload was checked by the existing benchmark. 35,198,976 measured
  echo frames completed. No observations/outliers were removed.
- Ryzen 9 7945HX, Linux x86_64, Rust 1.100.0, tungstenite 0.30.0, two Tokio workers,
  affinity to physical cores 2 and 4. SMT siblings are 18 and 20. Parent reported
  an unpinned Gradle run completing near the first-stage start; possible early
  overlap is retained, not excluded. Later Gradle work used other cores. This is
  an active workstation, not an exclusively reserved or frequency-controlled CPU.
- Initial measurements: 16:45:09–16:46:22 CET; finer: 16:46:52–16:47:23;
  final: 16:47:56–16:48:10, 2026-11-22. Raw records carry epoch timestamps.
- Saturated loopback binary echoes measure transport behavior, including both
  endpoints in one process. They omit production JSON parsing, native input calls,
  real traffic pacing, USB/Wi-Fi and device latency. Large-frame throughput is a
  synthetic stress case, not camera/display transport: those pixels do not travel
  through this input WebSocket.
- The client read buffer stayed 4 KiB. Only server read capacity varied; server
  message/frame limits stayed 64 KiB. No size is established as a global optimum.

## Initial 324-run sweep

Values are medians across 18 runs per size/workload. RTT is each run's p95 across
80,000 interactive echoes. Raw summaries also retain min, p10, p25, p75, p90, max.

| Buffer KiB | Interactive MiB/s | Interactive p95 µs | Small-burst MiB/s | Large-burst MiB/s | Interactive VmHWM KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4 | 37.15 | 56.86 | 131.50 | 2428.50 | 5292 |
| 8 | 37.37 | 54.32 | 129.08 | 2635.66 | 5250 |
| 16 | 36.98 | 54.90 | 106.94 | 2645.39 | 5322 |
| 32 | 35.95 | 57.04 | 121.46 | 2710.28 | 5328 |
| 64 | 33.90 | 61.48 | 114.09 | 2625.22 | 5426 |
| 128 | 31.83 | 65.98 | 103.41 | 2633.98 | 5604 |

Paired per-block 8/4 median ratios: interactive throughput 1.000
(bootstrap 95% interval 0.996–1.030); interactive p95 RTT 0.991 (0.928–1.010);
small-burst throughput 1.005 (0.874–1.108); large-burst throughput 1.082
(1.058–1.109). Ratios of medians differ from medians of paired ratios.

64/128 KiB regress interactive throughput and latency clearly in this sample.
32 KiB improves large bursts more, but its small-burst result and capacity cost
make it a weaker input-server compromise. 16 KiB has a marked small-burst penalty.

## Independent finer 144-run confirmation

| Buffer KiB | Interactive MiB/s | Interactive p95 µs | Small-burst MiB/s | Large-burst MiB/s | Interactive VmHWM KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4 | 38.78 | 51.94 | 137.16 | 2509.85 | 5262 |
| 6 | 38.30 | 52.75 | 122.40 | 2625.62 | 5282 |
| 8 | 38.79 | 51.67 | 130.16 | 2670.76 | 5268 |
| 12 | 37.71 | 53.30 | 114.12 | 2710.15 | 5306 |

Paired 8/4 ratios: interactive throughput 0.996 (0.991–0.998), interactive p95
1.008 (0.995–1.020), small bursts 0.982 (0.864–1.018), large bursts 1.054
(1.018–1.091). Interactive differences are below 1% by paired medians. 6 and
12 KiB did not offer a better overall tradeoff than 8 KiB in these workloads.

## Final small-burst confirmation, 60 pairs

4 KiB median throughput: 137.82 MiB/s; 8 KiB: 128.89 MiB/s.
Paired 8/4 median ratio: 0.951; bootstrap 95% interval: 0.917–1.010.
8 KiB wins 24 of 60 pairs. This suggests a small-message burst penalty rather
than a demonstrated gain; noise still admits parity. Final VmHWM medians were
4456/4484 KiB. Do not hide this result by pooling it into earlier stages.

Intervals use 10,000 paired-block bootstrap resamples, fixed seed 17380023,
of the median ratio. They are descriptive uncertainty estimates for this finite
workstation experiment, without multiple-comparison correction or cross-machine
inference. Three stages remain separate; initial candidate selection is explicit.

## Memory and ownership

8 KiB reserves 4 KiB more per admitted server connection. At eight connections,
configured capacity is 32 versus 64 KiB; at the production 16-handler ceiling,
64 versus 128 KiB (64 KiB extra). 128 KiB would reserve 2 MiB at that ceiling.
Whole-process VmHWM is independently observed residency, not configured capacity;
allocator noise, sample vectors and uncommitted pages prevent interpreting small
RSS differences as memory savings. The protocol message/frame limit remains 64 KiB.

Each run owns its loopback listeners, socket futures and task set. The existing
benchmark joins all tasks even on failure. The Python runner waits for each exact
child before admitting the next, captures its bounded one-result output, and uses
subprocess.run's kill-and-wait cleanup on exceptional cancellation. There is no
elapsed-time termination and no production PID interaction. All planned cases
finished; local artifacts remain owned by this experiment. Four local scripts
contain 12 functions, all measured complexity <=6 using the repository Python AST
checker (Python 3.12); benchmark/development scripts are coverage-exempt.

## Artifacts

Original execution directory: `/tmp/blent-buffer-sweep-20261122/`.
This retained artifact directory stores raw JSONL files compressed as `.jsonl.gz`;
extract them before running the original analysis scripts. Script paths preserve
the original experiment, and can be adjusted for a new owned execution directory.

- `metadata.json`: environment, binary/source hashes and full initial plan.
- `results.jsonl`, `summary.json`: initial measurements and spread/paired intervals.
- `confirmation.jsonl`, `confirmation-summary.json`: finer measurements.
- `small-confirmation.jsonl`, `small-confirmation-summary.json`: final 60 pairs.
- `warmups.jsonl`, `confirmation-warmups.jsonl`: retained excluded warmups.
- `run.py`, `confirm.py`, `confirm_small.py`, `analyze.py`: reproducible finite runner/analysis.
- `complexity.json`: per-function tool measurements.

Benchmark source SHA-256:
`4eccc821b6eeac548193b6d37fbcc1d563c4ca5b59e4b870dc1c84ab5e7bad91`.
Release binary SHA-256:
`6a926cef292e83151e8f5ddb03d10d55e561aba3a9d353c21f1eec3e18d38c76`.
