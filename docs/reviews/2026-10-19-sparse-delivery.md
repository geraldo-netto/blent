# T1107: sparse delivery qualification

The installed CPU H.264, VP9 and AV1 recipes deliver their first picture while
stdin remains open, preserve a static gap longer than one second, deliver resumed
motion and cursor-like updates without another input picture, and independently
decode the subsequent keyframe using its current configuration. This qualifies
the tested software framing path. Unchanged-frame suppression is still planned
under T1109–T1112; current production cadence is unchanged.

## Experiment and results

The permanent normal-suite tests use `blent::ffmpeg_args::Settings::arguments`
directly and the production `Packetizer`, including FramedAnnexB and IVF readers.
They do not reconstruct the production recipe in Python. Inputs are five bounded
64x64 NV12 pictures at the selected nominal 60 fps, quality 18, 20,000 kbit/s and
one requested encoder worker. Two deliberate 1.25-second gaps retain stdin open.
The selected input is `pipe:0`; no FIFO, EVDI, display or tablet is opened.

FFmpeg 6.1.1-3ubuntu5, libvpx 1.14.0 and the installed libx264/libaom builds were
used. Exact argument lists and source hashes are retained in the
[artifacts](artifacts/2026-10-19-t1107/README.md). One final local run measured:

| Encoder | First picture | Motion | Cursor-like update | Late keyframe | Static PTS gap |
| --- | ---: | ---: | ---: | ---: | ---: |
| libx264 | 61.2 ms | 0.25 ms | 0.09 ms | 0.29 ms | 1.317 s |
| libvpx-vp9 | 124.9 ms | 0.89 ms | 0.63 ms | 0.93 ms | 1.383 s |
| libaom-av1 | 61.8 ms | 0.43 ms | 0.30 ms | 0.92 ms | 1.317 s |

These are write-to-complete-production-packet timings for tiny synthetic inputs,
including startup for the first picture. They are not display, USB, GPU or battery
measurements, statistical performance claims, or qualification at other sizes,
profiles, worker counts or FFmpeg versions. Each picture has a two-second finite
delivery acceptance bound. Compiler and original-retirement observation have no
elapsed execution cap.

Each session verifies exactly increasing sequence and timestamps and the original
encoder generation. Initial, post-static and late-join keyframes decode in fresh
FFmpeg processes with exactly one NV12 picture, including chroma and a 16x16
cursor-like patch. The independent expected pixels allow at most eight byte
levels of compression error; this checks identity and recovery, not perceptual
quality. All five pictures emerge before any successor or EOF is supplied.

## Android audit and hosted evidence

`DecoderOutputWatchdog` counts admitted pictures and resets its outstanding count
on output. Elapsed idle time alone does not call `queued` or create a stall.
Four admitted pictures with no output still trigger recovery after the watchdog
window. `CallbackDecoder` schedules input expiry only for an actual pending
mailbox entry, with a fresh 200 ms deadline for each newly admitted input.

The new hosted callback test drains a picture, advances the clock by 60 seconds,
crosses a real handler barrier, and resumes input/output through the same codec.
The new pure watchdog tests cover 128 sparse completions at boundary and long
gaps, and independently retain rejection of uncompleted admitted work. Existing
callback, watchdog and negotiated-liveness tests run alongside them: 48 tests
pass, with zero failures, errors or skips. Callback/watchdog tests use hosted
API 27 and 34 environments; they do not execute physical MediaCodec hardware.

Transport liveness remains separate from video progress. The existing packet
reader accepts negotiated liveness without dispatching media, queuing a decoder
picture or generating a render acknowledgement. Such messages cannot qualify a
decoder that has not produced matched real-picture acknowledgements.

## Ownership, controls and remaining work

Native encoders and independent decoders use the existing `OwnedChild` admission
and `Scope::group` sponsor before exposing pipes. Each test retains the original
native completion observation through actual child/reaper settlement, including
partial startup and failed delivery/decode. A dropped session closes transferred
pipe views; private child buffers remain child-owned until original exit. No
native shared pixel pointer crosses the pipe. The Rust test runtime remains live
until the original group reports complete. Conditional eventual cleanup assumes
eventual scheduling and the installed trusted tools terminate after group stop;
this is not an arbitrary daemon/process-tree fence.

Android callback testing uses the existing original codec retirement barrier
before fixture surface release. Pure watchdog test state is confined to each
test invocation; its 128 iterations and bounded nanosecond values allocate no
native codec or persistent worker.

Six new Rust tests pass, including controls rejecting duplicate, backward,
foreign-generation, unkeyed and unconfigured output. A new permanent exhaustion
test first exposes overflow in the active qualification oracle; checked addition
then makes that same test pass. Existing framing tests pass nine cases, with one
pre-existing opt-in benchmark ignored; the ten IVF controls pass separately. No
existing test or exclusion was removed or weakened. Five architecture tests pass.
Source estimates cover 16 Rust and 55 Kotlin functions, maximum three; these are
repository parser estimates, not native SonarQube measurements (T1019).
No production function body changes, so this item adds no application counter gap.

GPU encoder families, HEVC sparse timing, physical Android decoders and the
optional in-process capture/encoder path remain unqualified by this experiment.
Unknown combinations must retain compatibility behavior. T1109 owns the portable
policy; T1110 owns native integration and explicit capability reporting; T1111
owns bounded user choices; T1112 owns joined synthetic comparisons. T492 physical
functional and T679 battery acceptance remain separate and unresolved.

Reproduce offline:

```sh
cargo test --offline --locked -p blent --bin blent t1107 -- --nocapture
cargo test --offline --locked -p blent --bin blent t448 -- --nocapture
cargo test --offline --locked -p blent --bin blent ivf -- --nocapture
android/gradlew -p android --offline --no-daemon --max-workers=2 \
  -I /home/netto/Desktop/projects/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest --tests com.blent.SparseDecoderWindowTest \
  --tests com.blent.CallbackDecoderTest --tests com.blent.DecoderWatchdogTest \
  --tests com.blent.VideoLivenessTest
```
