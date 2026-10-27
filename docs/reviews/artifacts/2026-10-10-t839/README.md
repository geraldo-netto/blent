# T839 — encoder profile numeric contract

Encoder profiles reject invalid numeric bounds before arithmetic. Permanent tests span registered encoders and preserve explicitly allowed low-level replay bitrates.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`Profile::new` rejects zero and excessive FPS/bitrate and quality outside the
shared supported range before arithmetic. Low-level FPS and bitrate accept
1 through their configuration ceilings (90 FPS, 60,000 kbit/s); quality remains
12 through 32. The public low-level adapter deliberately permits small replay
bitrates (including 200/500 kbit/s), while normalized user configuration and
private worker requests keep their stricter configuration minima.

The permanent `t839_*` tests first failed: the extreme libx264 bitrate panicked
on multiplication and the bounded grid accepted invalid values. The same tests
now pass, covering all eleven registered encoders across 3,960 numeric boundary
combinations, plus unknown names and the explicit overflow case. T373's previous
zero-FPS normalization assertion now tests the valid minimum; zero rejection is
covered by T839. Existing low-bitrate native encoder tests are retained.

All 217 common library tests pass. A fresh offline LLVM collection of
`encoding::` measures both changed production functions at 100% executable-line
coverage; complexity is at most five. Raw counters and red/green logs accompany
this report. These are scoped results, not a whole-project coverage claim.

Reproduction:

```sh
CARGO_NET_OFFLINE=true cargo test --offline --locked -p blent-config --lib
CARGO_LLVM_COV_TARGET_DIR=/tmp/blent-t839-cov cargo llvm-cov --offline --locked \
  -p blent-config --lib --lcov --output-path /tmp/blent-t839.lcov -- encoding::
```

</details>
