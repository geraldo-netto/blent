# T750 snapshot and damage-order research — 2026-10-09

CPU-backed snapshot and damage-order trials passed independent frame checks. The GL path failed before conversion and remained unqualified; no new Blent corruption or general driver mitigation was established.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

No new Blent snapshot corruption reproduced. CPU-backed paths pass bounded
native acceptance. GL/import acceptance fails before conversion and remains
tracked under T749; T750 now names that concrete prerequisite instead of a
vague request for more validation. Neither EVDI nor libevdi was edited, patched,
installed or reloaded on the host. No downloads or network requests occurred.

## Mechanism and applicable protections

Research used cached primary-source issue bodies/comments and stock source from
the earlier October 9 review; upstream status was not refreshed online.
[Issue 511](https://github.com/DisplayLink/evdi/issues/511) describes separately
updated framebuffer, damage and vblank state. A valid dirty rectangle paired with
a different commit's framebuffer could miss changes or yield a partial snapshot.
The report does not establish that every compositor or Blent run suffers this.
[Issue 512](https://github.com/DisplayLink/evdi/issues/512) separately concerns
capture-triggered flip events and excessive rendering cadence. Cached
[PR562](https://github.com/DisplayLink/evdi/pull/562) and
[PR594](https://github.com/DisplayLink/evdi/pull/594) propose kernel-side changes;
none are used here.

Blent already bounds immediate/asynchronous request cadence, acknowledges GRAB
before another request, and retains bounded watchdog/fallback recovery. The
stock GRAB implementation can take/send a pending flip even with zero damage;
Blent's fallback therefore matters when an acknowledgement arrives late. A zero
rectangle result is not a success/failure or generation token: libevdi also
returns zero on grab errors. Re-registering a userspace destination merely
changes libevdi's buffer list; it does not request a complete scanout copy.

Stock libevdi provides dirty-only copies, no coherent-snapshot identifier and no
public forced-full-copy/query/cancel operation. Converting all NV12 pixels again
cannot repair source pixels never copied correctly. A second GRAB can consume
empty or newer damage; equality of two reads is not an atomicity guarantee.
Black-image rejection cannot distinguish an actual black desktop from broken
readback. These are not safe general Blent mitigations, so none were added.

Blent captures its own `CLOCK_MONOTONIC` time after GRAB in `capture.c`, propagates
it through frame exchange/shared-slot metadata, and does not derive that time
from DRM/Wayland presentation feedback. Existing encoder timestamp handling is
also separate from the compositor's feedback. The zero presentation timestamps
observed below therefore do not replace Blent capture timestamps.

## Native method

Disposable QEMU/KVM guest, kernel 7.0.0-34-generic, two pre-created EVDI devices,
stock EVDI1.15.1 or cached distro1.14.15, same stock libevdi1.15.0. No guest network
device. GL uses virtio/virgl with the existing host render node; it is not a
native AMDGPU/Vulkan compositor acceptance run. Source buffers are immutable
until producer exit. Tests use 640×480 grayscale patterns: a frame ID and its
complement plus 80 independently updated tiles. Each of 240 updates changes the
ID strip and one tile. Every checked image must match one complete generation;
skipped intermediate generations are allowed, mixed/backward frames are not.

A guest-only preload observer calls the original stock libevdi functions and
checks each nonempty registered-buffer capture before Blent conversion. The
FIFO consumer checks complete NV12 frames and final convergence to frame 240.
The shared pattern oracle is in `scripts/benchmarks/evdi_snapshot`; retained logs
contain commit, captured-ID, flip and presentation-feedback observations.

| Run | Producer | Valid raw checks | Checked NV12 frames | Result |
|---|---|---:|---:|---|
| sparse | stock1.15.1 KMS, two damage rects, 240 flip events | 241 | 295 | pass |
| fragmented | stock1.15.1 KMS, 63 overlapping rects | 241 | 295 | pass |
| distro-fragmented | distro1.14.15 KMS, same fragmented case | 241 | 295 | pass |
| wayland-pixman | stock1.15.1, Weston13 DRM/pixman, SHM client | 242 | 280 | pass |
| wayland-stock / wayland-diag | stock1.15.1, Weston13 DRM/GL, same client | 0 | 0 recognized | fail |

Every passing run observes all 241 IDs (initial zero plus 240 changes), no corrupt
snapshot, no backward ID, and final frame 240. NV12 counts include repeated
keepalives: 1,165 validated output frames are not 1,165 distinct source changes.
Fragmented KMS runs request feedback for the first 120 commits and omit that
optional event for the remaining 120. Kernel serialization still paced that
second phase; these results do not claim forced overlapping atomic commits.

The initial `stock-a` probe counted Blent's intentional uniform gray startup
placeholder as invalid. Inspection of `on_mode_changed` established the existing
contract. Subsequent fixture runs accept only that exact placeholder before the
first captured pattern. Compositor runs additionally permit its background
before a recognizable pattern; no recognized pattern always fails. Malformed
frames after recognition never get ignored. This is fixture calibration, not a
production fix or weakening/deletion of an existing regression.

## Compositor findings

Weston pixman yields 241 ordered presentation notifications, all 241 frame IDs
in captured pixels, and 280 correct NV12 frames. Callback observations are
correlated by the embedded identity, not inferred from matching frame counts.
All 241 reported presentation timestamps and sequence values are zero; the same
occurs in both GL runs despite a monotonic clock ID. These are virtual-output
notifications, not Android/physical-screen presentation measurements.

Both GL runs produce no recognizable frame ID at the stock libevdi output.
Diagnostic samples from seven rows and 40 columns are all zero in the inspected
blue byte. Pixman's corresponding samples carry the intended identifiers and
tile values. Thus the GL result cannot validate snapshot ordering, and a Blent
NV12 conversion change cannot recover the absent source pattern. At the time of these runs the precise render/export/import/readback cause was
unassigned. The [subsequent T749 controls](../2026-10-09-t749-readback/README.md)
isolate virtual GPU CPU visibility: full GBM readback passes and partial readback
exposes only the mapped rows. That active intervention does not certify the
original path or establish an issue511 reproduction.

## Permanent owned-code regression and limits

`host/tests/snapshot_order.c`, invoked by the normal helper test target, covers
240 sparse updates coalesced eight at a time, alternating rectangle order,
duplicate empty-ready callbacks, skipped writer claims and retained immutable
leases. The entire latest NV12 image must match the independent generation
contract. Thirty-two bounded pixel mutations must be detected by the checker.
This passes on existing production behavior; no new runtime fix was warranted.
GCC and Clang ASan/UBSan pass. Full normal native suite passes and all 207
production functions retain at least 80% individual executable-line coverage.
Changed/new fixture functions satisfy cyclomatic complexity ≤9. Native fixture
programs also reject execution on this unmarked host with exit 64.

No result proves the absence of every kernel interleaving or imported-GPU race.
The exact remaining prerequisite is a working unassisted GL/import path (T749),
then repetition of the frame-identity test on that path. Current evidence supports
the tested CPU-backed paths; it does not justify a blanket release-failure claim
based only on an unconfirmed upstream race. No heuristics or external patches
were introduced to make a failing acceptance run appear successful.

</details>
