# T787 — shared CPU input for stock baseline VAAPI

Shared CPU input reduced measured conversion/transport/upload cost on the tested VAAPI setup. Upload remains, selection stayed explicit, and native acceptance on two AMD nodes is not all-GPU or end-to-end qualification.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Implemented a Linux hardware-frame adapter in Blent's optional in-process
encoder. `h264_vaapi_baseline` now resolves to stock `h264_vaapi`, retaining
Constrained Baseline/CAVLC, CQP, zero B-frames, async depth one, BT.709 and
next-frame IDR requests. `vaapi_device` is forwarded from session configuration.
Other VAAPI profiles remain unsupported by this adapter.

The existing C producer already converts directly into leased memfd NV12 slots.
The new adapter validates immutable buffer-backed plane extents, uploads into
independently owned VAAPI surfaces, and retains FFmpeg references through codec
retirement. Allocation, device-open, upload and metadata-copy errors propagate;
partial surfaces and pools release their references. Native APIs stay within the
Linux encoder boundary; portable policy remains in `common/src/encoding.rs` and
the independent raw-frame contract is unchanged. No dependency was patched.

Selection remains explicit: build `inproc-encoder`, choose
`h264_vaapi_baseline`, leave `encoding_gpu` empty, set `vaapi_device`, and choose
`raw_transport = "shared_memory"`. Auto retains FIFO for VAAPI; the default CLI
build is unchanged. Named GPU selection in the optional build still falls back
to software. Stock runtime device/size/codec constraints can reject initialization.
The current driver requires at least 128×128 for encoding. An initially four-surface stock pool
supports the synchronous adapter; raw slot capacity remains configurable, 2–8.

## Measurements and limits

`boundary.c` isolates conversion, cross-process raw transport, CPU-to-GPU upload
and synchronous H.264 packet delivery. It includes the production C conversion
kernel and stock-API hardware encoder helpers. Both cases use the same hardware
pool, encoder settings and pixels. Shared input uses an anonymous shared mapping;
FIFO input uses a 4 MiB pipe and a consumer CPU frame. It does **not** run the
final Rust orchestration, memfd descriptor protocol, USB transport or Android.
The final Rust adapter is validated separately by native encode/decode tests.

1280×800, four conversion workers, QP18, nominal 60 FPS, 150 frames/run with 30
warmup frames, 10 ms between frames. Five alternating pairs on renderD129:
median run p50 **3835.748 → 3176.556 µs**; median p95 **4644.445 → 3541.879 µs**.
Four pairs improved; the last, slower pair slightly regressed. All ten H.264
streams had identical SHA-256 hashes (`environment.json`). `latency.json` retains
individual timings. These are local synthetic measurements, not a claim about
complete stream latency or all GPUs. They justify an explicit shared-input path,
not changing automatic admission or the default encoder.

`boundary-resources.c` repeats the same probe with getrusage CPU and separate
parent/child peak RSS measurements, including startup and warmup. See
`resources.json`. Median total CPU fell **595.066 → 433.433 ms** per 150-frame
run (~27%); corresponding median p50 was **3636.994 → 2875.957 µs**. Median
parent peak RSS was 21,740/22,008 KiB (FIFO/shared), child 71,796/72,024 KiB.
These small RSS differences do not measure kernel pipe storage or GPU allocations;
shared mappings are also counted in each process.
The shared ring may retain more CPU memory than one FIFO frame: four 1280×800
slots contain 6.144 MB of pixels plus page padding. Users control slot count.

CPU-to-GPU upload and stock EVDI readback remain. Direct mapped GPU surfaces need
a different producer stride/history/lifetime contract; this change does not
claim CPU-to-GPU zero-copy or implement cross-device imports. `mapping.json`
records mapping availability only. No live desktop,
installed daemon, Android application or dependency was modified for testing.

## Permanent validation

`red.log` records baseline VAAPI admission failing before implementation. The
normal suite retains T787 tests for bounded plane strides/offsets/row extents,
invalid dimensions/device paths, missing backing buffers, allocation failures,
I/O failure during upload, failed metadata copy, repeated attachment, surface
retention and source immutability. The portable foreign-API seam always runs and
uses real AVBuffer/AVFrame ownership with CPU surfaces; it does not stand in for
native-driver acceptance. Device-loss error propagation is injected as EIO;
physical device removal was not performed.

With `BLENT_TEST_VAAPI_DEVICE` set, tests additionally exercise stock hardware
upload/readback and the real C memfd producer (compiled with ASan/UBSan), encode
read-only slots and copied input, compare complete packets, decode pixels, and
check immediate IDRs. Native acceptance passed both local AMD render nodes,
at 128×128, 1280×800 and padded 1282×802. Without the environment variable, these
native additions do not run; that is not evidence of another machine's support.
Existing T418 final-reference retention, full-ring exhaustion, malformed/stale
messages, resize, cancellation and peer-crash regressions remain unchanged.
Configuration, doctor and tablet-selection tests reflect the new baseline
capability while retaining rejection of unsupported profiles.

Reproduce offline with a matching locally installed stock FFmpeg development SDK:

```sh
BLENT_TEST_VAAPI_DEVICE=/dev/dri/renderD129 cargo test --offline --locked \
  -p blent --features inproc-encoder --bin blent encoder::
cargo test --offline --locked -p blent-config encoding::
```

Compile either boundary probe with `cc -O3 -ffunction-sections -fdata-sections`,
include `host/evdi`, `host/gpu`, `scripts/benchmarks`, stock FFmpeg/VA/X11 and DRM
headers; link `-Wl,--gc-sections -lavcodec -lavutil -lva -lpthread`.
Run `probe /dev/dri/renderD129 MODE 4 output.h264`, MODE 0 for FIFO, 1 for shared.
Keep trials sequential and alternate order. No downloads are needed when the
matching SDK is already cached.

All 297 host library tests and 270 host binary tests pass in the optional build;
existing opt-in benchmarks remain excluded (three library, two binary). The final
constructor cleanup additionally passes all 37 encoder tests. Nine common policy
tests and the default-build capability regression pass. `build.log` records a
warning-free non-test build check. Native LLVM counters show **16/16** new or
changed production functions above 80% individually (minimum 96.49%; all seven
hardware-boundary functions 100%). See `coverage.json` with source hashes and
`coverage.lcov.gz`. Production cyclomatic complexity remains at most nine.

Coverage was collected offline with `CARGO_LLVM_COV_TARGET_DIR=/tmp/blent-t787-cov`,
`BLENT_TEST_VAAPI_DEVICE=/dev/dri/renderD129`, and `cargo llvm-cov --offline --locked`
for `-p blent --features inproc-encoder --lib --bin blent` and `-p blent-config --lib`,
exporting LCOV. Native D128 acceptance is retained separately. SDK library/include
paths are environment-specific and must match the stock runtime libraries.

</details>
