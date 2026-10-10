# T814 retirement evidence

All runs offline. No production services reloaded, physical devices opened, or
dependency sources modified. Tests use owned children, temporary FIFOs, held
in-process adapters, and private PipeWire servers with synthetic PCM.

Seven retained failing logs precede their respective behavioral changes:
held config commit, post-abort native join, accepted signal without reap,
erased encoder cleanup error/panic, erased session cleanup error/panic,
thread-local destruction stalling sibling joins, and held dependent destructor.
For `cleanup-red`, the old discarded-outcome behavior was temporarily restored
in the refactored owner before running its new permanent regression. The other
red logs exercise the code in place before their respective fixes. The bounded
kill fixture changes kill(2) only in its disposable test process and always
self-exits; it is not evidence of a real unkillable kernel process.

`coverage.log.gz` contains the default common/host library, host binary,
portable session, failure supervision and shutdown-deadline runs: 954 passed,
seven existing ignores. `stop-coverage` adds the synchronous-stop ownership test.
The installed offline SDK supplies stock native headers for 38 optional capture
checks and ten audio unit checks. SDK-native audio acceptance could not find the
PipeWire executable (`sdk-native-audio`); the same two permanent acceptance tests
passed on the host using SDK-built native-audio-only binaries and installed
PipeWire tools (`host-native-audio`). They cover microphone and stereo speaker
CLI transport, clocked/ordinary PCM, node teardown and graph loss, with private
runtime directories. No production PipeWire node is used.

`functions.json` max-merges the two LCOV inputs for the same production source
bytes: all 213 maintained production functions meet individual 80% line coverage
and complexity <=9. Sources remained unchanged throughout collection; one extra
test was added and collected cumulatively without changing production code.
`source-manifest.json` records final source/test hashes. New module-export and
existing entry-point test-wiring files do not change production functions.
Windows-specific job/retirement execution and counters remain T737. Missing SDK
mount/bus/PipeWire tools remain T835; no broad all-features pass is claimed.

Collection used `cargo llvm-cov --offline --locked`, with dedicated target roots
`/tmp/blent-t814-target` and `/tmp/blent-t814-sdk-target`. Default collection:
`-p blent-config -p blent --lib --bin blent --test session_failure
--test portable_session --test shutdown_deadline`. SDK capture uses
`-p blent --all-features --bin blent -- capture::`; audio units use
`--all-features --lib -- audio::`. Native acceptance uses the unchanged
`host/tests/audio_native.rs` built with `native-audio`, then executed on the host
with `LLVM_PROFILE_FILE` pointing into the SDK coverage target. Four existing
host-boundary guards and 128 no-default common unit tests also pass.

This validates control return and ownership, not a throughput improvement,
real-time scheduling, cancellation of started native work, or final daemon exit
under an indefinitely stuck in-process codec (T819).
