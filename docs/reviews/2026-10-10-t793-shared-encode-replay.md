# T793: current-source shared-encode replay repair

The standalone replay now builds offline against the current encoder and raw
transport adapters. Its generated crate copies the nine private native modules
(including `encoder_hardware.rs`) and imports shared media/configuration/latency
contracts from the actual `blent` library. This preserves the type identity used
by the capture contract, rather than copying stale implementations of shared
contracts. It seeds resolution from the repository lockfile and honors the same
absolute Cargo target directory for building and running.

The replay uses the production `Encoder::with_device` and
`Input::open_with_nonce` constructors. The existing software settings remain:
libx264, one codec worker, quality 18, 60 fps, 60 warmup frames and 60 cooldown
frames. The device argument does not open a GPU for this software encoder.
Errors while attaching the inherited raw socket propagate instead of being
ignored.

Fresh execution also exposed a replay shutdown race: the C producer closed its
shared socket with unread RELEASE notifications, allowing the consumer to see
`Connection reset by peer` before reading the final queued frame. The observed
failure encoded 122 of 123 frames. The producer now closes its timestamp pipe
first and services the shared socket until the consumer drops its encoder and
input owners, with a two-second deadline. This only changes the benchmark
producer; production EVDI and dependencies are untouched.

## Permanent regression and evidence

`host/tests/shared_encode_replay.rs` runs
`host/tests/shared_encode_replay.py` in the normal Linux `inproc-encoder` test
configuration, which the existing all-features CI job runs. Missing native
build/decoder tools fail this configuration; the test does not silently skip.
The generator builds with Cargo `--offline`. Under cargo-llvm-cov, the wrapper
isolates this generated benchmark build from the outer production coverage
counters because copied sources live in a temporary directory. Normal production
coverage remains in the outer suite.

The regression was added before the fix. Its first run failed compiling the
fresh generated project because `src/encoder_hardware.rs` was absent. After the
module/constructor fix, the same test exposed the shutdown failure above.
After the teardown fix, both direct Python execution and the native Cargo test
passed. An isolated reconstruction of the original producer close reproduced
that intermittent failure on the second unchanged regression run: the shared
66x50 stream delivered 122/123 frames and failed with ECONNRESET. The raw log,
fixture and substitution script are retained; repository sources were not
reverted and no timing hooks were introduced.

The test compares FIFO and shared-memory paths at 64x48 (aligned rows) and 66x50
(padded codec rows). Each path produces all 123 frames, including warmup and
cooldown. All three measured sequence IDs are present; H.264 bytes and decoded
YUV420 bytes match exactly between paths for each geometry. The test checks the
decoded byte count and that the first and last patterned frames differ.

Run in an environment with cached dependencies, stock libavcodec development
headers, FFmpeg with libx264 and a C compiler:

```sh
cargo test --offline --locked -p blent --features inproc-encoder \
  --test shared_encode_replay -- --nocapture
```

Local validation used the already installed Freedesktop SDK 25.08 with
`flatpak run --unshare=network`; nothing was downloaded. Cargo integration:
1 passed, 0 failed. The associated Python fresh build and four replay runs
finished in 19.4 seconds on this machine; this is test runtime, not a throughput
or latency claim.

## Scope of the evidence

This is a conversion/raw-transport/codec correctness smoke test. It directly
invokes the encoder adapters and does not exercise the production encoder-worker
process orchestration, EVDI capture, GPU upload, Android rendering or USB. The
archived T418 performance measurements remain historical. This repair establishes
that fresh current-source replays build and produce equivalent output; it does
not replace those measurements with new performance claims.

The existing C benchmark regression suite also passes all five tests, including
ASan/UBSan invalid-argument and stride checks. Each of the ten producer function
bodies independently parses and stays at complexity nine or below; the new
`finish_shared` scores two. The whole-file parser rejects the existing valid
`__has_include` header guard; T843 tracks that reporting gap, so no whole-project
complexity pass is claimed. Benchmark code remains outside the maintained
production per-function coverage requirement; all existing tests are retained.

[Raw red/green logs, sanitizer results, source hashes, and function-body metrics](artifacts/2026-10-10-t793/)
are retained with this review.
