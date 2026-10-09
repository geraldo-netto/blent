# Android review implementation

## T829: decoder replay generator prerequisite

The current-source offline decoder benchmark failed to compile because its
source list omitted `DecoderDiagnostics.kt`, while DecoderSession already used
its types. `test_decoder_diagnostics_dependency.py` failed before the source-list
fix and passes afterward. Generated `compileDebugKotlin` passes offline. This
repairs the benchmark only; production behavior is unchanged.

## T822: independent wire contracts

Protocol limits, formats, endpoint validation, selection values and standard
profile vocabulary now live independently of receiver/codec/Intent owners.
JSON, Intent and MediaCodec mappings are explicit adapters. Packet diagnostics
are injected by the receiver; packet readers contain no Android calls. Native
capability checks and existing wire behavior remain intact. The standalone
benchmark copies the extracted contracts and selects the matching JSON adapter.

Permanent architecture regression failed against the original dependencies and
passes after extraction. Plain-JVM contract tests cover payload borrowing,
selection receipts and invalid identifier/profile/endpoint boundaries. The wire
contracts/readers also compile with only Kotlin/JDK, without Android or stubs.
Offline Android suite: 731 tests, zero failures; JaCoCo/PSI gate: 786/786
maintained methods reach at least 80% individually. Production Kotlin cyclomatic
complexity remains at most 9. Generated decoder Kotlin compilation and benchmark
source-dependency regressions pass. No native installation or download.

## T810: portable resource lifecycle policy

Audio/camera bindings now depend on independent endpoint/preferences/state values
and a LifecycleRun interface. Android construction adapters own Context,
permission checks, native capture/resource types, services and preference storage.
Compose observes immutable StateFlow snapshots. Cancellation remains distinct
from confirmed retirement; generation checks and borrowed native lifetime gates
remain. Camera rotation is sampled at actual execution, after earlier retirement.

The architecture regression fails on the previous Android/Compose dependencies
and passes now. Plain-JVM lifecycle tests exercise resource delegation, state,
late status rejection and Stop; policies also compile against Kotlin/coroutines
without Android or Compose. The full offline Android suite passes, including
existing ownership/permission/service/recreation tests. All 813 maintained
Android methods pass individual 80% coverage; production complexity stays <=9.
T807/T806 retain the known cleanup-error and unbounded-waiter behavior to fix next.

## T807: contained cleanup failures and uncertain retirement

Independent Activity teardown actions continue after ordinary failures. Audio
and camera service-stop errors remain visible in their owner state without
escaping a root coroutine; unconfirmed background-service identity is retained.
Native camera close attempts retain reverse order, report incomplete cleanup,
retain uncertain handles and do not automatically retry their destruction.
A portable retirement guard keeps a failed native owner reachable and rejects
replacement. Atomic state updates preserve concurrent cleanup observations.

Decoder retirement had the same admission flaw: swallowed release errors removed
the process-wide gate. Failed MediaCodec release now retains that owner/gate;
confirmed release after a failed stop still permits replacement. Fatal errors
are not consumed by the new cleanup boundaries.

Eight new owner/Activity/native-close regression cases failed before the fix.
The added codec test failed on both SDKs when release failure reopened admission;
normal release controls already passed. All now pass. Existing reverse-order,
late-startup and T707 tests retain their cleanup assertions and additionally
require visible errors and contained exceptions. Plain-JVM tests cover admission,
late/interrupt failures, independent cleanup and fatal-error propagation.
Full offline Android suite: 749 passing tests; 834/834 maintained methods pass
individual 80% coverage; production complexity remains <=9. No physical native
failure was induced. T806 still owns bounded requests/deadline reporting and
T798 still owns the UI retirement wait.

## T806: bounded admission and observable native waits

Audio and camera share a portable ResourceLane: one active/retiring owner and one
latest pending request. Superseded requests allocate neither native owners nor
predecessor jobs. Stop discards the pending request; only confirmed retirement
admits a successor. Failed cleanup retains the existing quarantine. A five-second
independent deadline reports stalled cleanup without freeing native memory.

Decoder startup/retirement now publish process-wide pending/stalled observations,
visible across presentation recreation. Camera callback leases share a startup
deadline across recovery attempts; missing callbacks remain visible even before
Stop. The last genuine callback return clears the startup wait. Camera destruction
stays on the Android adapter's I/O dispatcher. Cleanup deadlines are notifications,
not guarantees that an in-process native hang or crash can be recovered.

The two original storm regressions failed before the change. They now exercise
128 replacements, latest-request selection, Stop/shutdown, bounded child jobs
and no queued resource construction. Separate regressions failed for unobservable
decoder waits on both SDKs, missing camera callback deadlines, and Stop hiding an
already-stalled owner. Permanent tests also cover pre-dispatch cancellation,
failed construction/destruction, timeout bounds, repeated lease returns and
progress of an independent lane while another remains stalled. Existing T707,
T732 and cleanup-failure regressions remain enabled.

Offline validation: 767 tests pass; all 864 maintained Android methods reach
80% individually; production cyclomatic complexity stays <=9. Generated decoder
Kotlin compilation and source-boundary/dependency tests pass. No downloads or
physical native hangs were induced. T798 still owns the UI retirement wait.

## T798: asynchronous decoder retirement from UI paths

DecoderSession.releaseCodec no longer joins the native retirement thread while
holding the receiver monitor. It still detaches the decoder, closes admission,
retires timing/callback state and delegates destruction to CodecLifetime.
Borrowed operations retain ownership, and the process-wide gate continues to
reject replacement until actual release. Failed setup may still await cleanup
on its existing I/O worker; that wait is outside the UI path.

The permanent main-looper regression failed at 502 ms on SDKs 27 and 34 before
the change. The same test returned in under 1 ms after the change, with its
borrower still held and replacement admission closed. These are host Robolectric
measurements, not a physical-device latency claim. Existing native-release-count
and replacement fixtures now explicitly await actual retirement before checking
completed destruction or reusing a fixture; no lifetime assertions were removed.
The focused decoder/callback/direct-input suite passes all 134 cases.

Replay workers also used the former synchronous retirement contract. Their
controlled-restart/receipt regressions failed after the UI join was removed;
replay now waits for process-wide retirement with cancellation and a five-second
deadline before native setup. Release completion means confirmed native release,
not the final bookkeeping instruction on its cleanup thread. The global gate
still stays closed until bookkeeping removes that owner. Replay setup timing
now includes any pending retirement wait; historical setup-only samples must
not be compared as if that waiting interval had always been included. Generated
bridges preserve older synchronous decoder sources.

Full offline validation: 770 tests pass; all 864 maintained Android methods
reach 80% individually; complexity stays <=9. Generated replay Kotlin compiles,
and replay/boundary tooling tests pass. No downloads or device installation.

## T799: generation-aware Surface wakeup

VideoReceiver now waits on a conflated StateFlow notification instead of a
200 ms timer. Readiness, destruction and session Start/Stop publish changes;
every observer checks current generation and readiness, including immediately
on subscription. Native setup remains on the I/O worker with its existing
Surface-identity and retirement checks.

All six deterministic regression cases failed before the fix on SDKs 27/34.
Their scheduler never advances timers: readiness-after-wait, readiness-before-
wait, destruction before dispatched wakeup, recreation and Stop now complete
correctly. Invalid/stale generation boundaries do not park a waiter. The existing
sampler fixture's synthetic private generation change now publishes a normal
lifecycle wakeup; its job/state assertions remain unchanged. This removes up to
one polling interval from readiness latency; no steady-state FPS gain is claimed.

Full offline validation: 776 tests pass; all 865 maintained Android methods
reach 80% individually; complexity stays <=9. No downloads or device changes.

## T802: request-scoped decoder metadata

One inventory enumeration per report; compatible entries reuse native capability
objects for details. Primitive profile/level keys deduplicate before support
queries; formatting stops at 32 supported profiles and 16 unique decoder rows.
Unknown/unsupported entries do not consume these limits. MIME arrays avoid tiny
set allocations; explicit bounded loops avoid sequence overhead. Exact format
checks, order, unknown optional values and fresh per-request inventory remain.

Counted inventory/profile regressions failed before implementation (four SDK
cases), then passed alongside exception, invalid integer, duplicate, changed
format and supported-cap boundaries. Full offline validation: 786 tests pass;
875/875 production methods reach 80% individually; complexity remains <=9.

[Native measurements](artifacts/2026-10-10-android-metadata/README.md): final
fresh-process trial warm report medians improve 8.3%, 22.5%, 31.4% and 57.7% for
four formats, with identical payloads. Cold-start ranges overlap. A sequence-based
trial regressed the common small format and was replaced before acceptance.
Isolated local benchmark packages only; no downloads or production app reload.
The global query isolation problem remains T808.

## T808: bounded native metadata admission

A portable NativeQueryLane admits one finite native call and no waiting queue.
The shell inventory receiver and control capability reports share that lane.
Coroutine cancellation and the independent five-second deadline stop waiting,
while the actual native owner retains admission until it returns. Busy/expired
control reports are explicitly unavailable with no positive capabilities; shell
reports retain their versioned unknown response. BroadcastReceiver uses goAsync,
and always finishes its pending result. ControlSession retains its socket/request
checks before publishing, so late native results cannot reach replacements.

Cancellation and blocked broadcast dispatch regressions failed on both SDKs
before the change; both now pass. Permanent tests cover blocked native ownership,
200 rejected replacement requests, timeout, cancellation, recovery, exceptions,
executor rejection, invalid deadlines, Stop/reconnect and late generations.
Existing ordered-broadcast assertions now await asynchronous completion.

Full offline validation: 798 tests pass; 889/889 methods meet 80% individually;
complexity <=9. Generated replay/metadata APK compiles with the new dependency;
boundary/generator tooling passes. Four native report payloads on the SDK36
USB tablet match T802 exactly (isolated benchmark package). No native hang is
claimed reproduced; latch-based fault injection proves caller isolation. Native
process crashes remain outside this thread boundary and are evaluated in T809.

## T803: control capacity and render-lock independence

The accepting slow-drain replay confirmed 574,100 queued bytes and up to 14,830 ms
delivered event age over 20 simulated seconds. A 64 KiB default budget now retires
that whole uncertain connection at 2,280 ms, with a 65,536-byte peak and delivered
age <=1,700 ms. [Deterministic results](artifacts/2026-10-10-android-control/slow-replay.json).
These are synthetic input/mouse/ACK queue measurements, not a live latency claim.

An independent byte-admission contract uses overflow-safe subtraction and UTF-8
wire sizes. User-facing limits range from 8 KiB to 1 MiB and persist locally;
tradeoffs and decoder-report implications are documented. Authentication and
settings keep their order. Overflow uses existing whole-session retirement,
which closes transport and releases uncertain host gestures, never replaying
partial gestures on a new connection. The limit applies on new sends and is not
a time-based delivery guarantee.

Decoder render notification captures immutable sequence/time/decoder identity
under its monitor, then invokes consumers outside it. The receipt carries a
lock-free current-owner check; control admission checks both that owner and the
captured connection generation after acquiring its own lock. Slow control sends
therefore cannot hold the receiver monitor. No additional callback worker or
unbounded message queue is introduced.

Both behavioral regressions failed on SDKs 27/34 before changes. Permanent tests
cover exact UTF-8 boundaries, invalid/overflow values, retired receipts, reconnect
while awaiting the control lock, settings persistence/range and existing ordered
handshakes/rejection recovery. Current and historical replay APKs both build
through the generation adapter. Full offline validation: 810 tests pass;
903/903 methods meet individual 80% coverage; complexity <=9. Tooling tests pass.

## T795: identity PCM ownership transfer

Aligned zero-ppm blocks transfer their removed PCM array directly to the output;
fractional phase, partial offset and signed-rate correction keep interpolation.
Offer explicitly transfers ownership; poll removes the complete block before
returning its mutable array. Timestamp/discontinuity semantics are unchanged.

Three permanent tests failed before the optimization, then passed: exact sample
and ownership transfer, seeded flat-sample interpolation reference with signed
rates/phase/offset, and overflow/expiry/discontinuity/invalid lengths. Existing
native-clock reset/epoch/boundary tests remain. Full offline validation: 813
tests pass, 904/904 methods meet 80% individually, complexity <=9.

[ART measurements](artifacts/2026-10-10-android-audio/README.md) on SDK36: compiled
isolated debug-APK median drops from 194.163 to 55.435 µs/aligned block and from
2211.840 to 98.304 allocated bytes/block, with identical checksums. Signed-rate
replays count actual transfer frequency and preserve the fractional fallback.
These synthetic CPU/allocation results do not measure native playback or physical
clock-session frequency. No dependency downloads or production app reload.

## T796: unchanged-window drift cache

AudioClockDrift retains the last successfully validated source/destination rate
pair and integer correction. Freshness is checked before cache lookup on every
call; clear resets both windows and cache keys. One-sided updates recompute exact
BigInteger arithmetic, including the rational 1000 ppm rejection bound. Invalid
results are never cached. No unchecked wider multiplication was introduced.

Permanent expiry-without-progress, one-sided window, unavailable, epoch/reset,
clear and repeated exact-boundary/rejection tests passed before this optimization
and after it. Full offline validation: 816 tests pass, 904/904 methods meet
80% individually, complexity <=9.

[ART replay](artifacts/2026-10-10-android-drift/README.md): wall median improves
13.224 to 9.967 µs/call and allocation 632.422 to 16.384 bytes/call with equal
checksums. Absolute CPU saving is small; no physical audio latency gain claimed.

## T797: PCM staging, typed output and metadata scratch

AudioPackets reuses its bounded byte staging/view per connection, bulk-converting
into a fresh owned ShortArray only after complete payload receipt. Failed reads
do not publish packet sequence state or partial PCM. Direct per-sample parsing
was measured and rejected. The portable OutputTrack contract now explicitly uses
sample counts/offsets and a nonblocking array borrow; AndroidSpeakerTrack uses the
stock typed AudioTrack write. AudioOutput no longer serializes PCM into a second
byte array, and still validates stereo-aligned partial/zero/dead writes.

Native speaker/microphone owners reuse AudioTimestamp scratch while returning
immutable samples. Clock metadata reads/writes use three network-order counters
directly; validity is checked before publishing outbound packet bytes. Existing
byte-format helpers and wire compatibility remain covered.

Five SDK regression cases failed before these changes (payload staging, typed
native writes and timestamp scratch). Permanent ownership, full-sample equality,
unit/boundary, cancellation, invalid-native-count and existing fragmented/atomic
packet regressions pass. Robolectric lacks short-array JNI support: its fixture
now routes typed writes into its existing byte listener/head-counter model,
retaining the native Stop/release assertions. Actual typed writes also pass on
the tablet. Full offline validation: 820 tests pass; 906/906 methods meet
80% individually; complexity <=9. Boundary/generator tooling passes.

[Incremental ART measurements](artifacts/2026-10-10-android-pcm/README.md), after
T795/T796: packet median 168.112 to 139.464 µs and 4358.144 to 2195.456 bytes;
portable output 63.639 to 54.585 µs. Per-sample parsing regressed ~44% against the
accepted bulk path. Silent native output shows 500.659 to 412.508 ms median thread
CPU per 100 blocks, equal native buffer/played counters and zero underruns. No
physical latency or end-to-end zero-copy claim; no downloads or app reload.
