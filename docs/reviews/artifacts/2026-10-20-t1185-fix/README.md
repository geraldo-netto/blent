# T1185: keep retained EVDI output progressing during media pause

## Change and contract

The original helper stopped every pixel grab while retaining a connected EVDI
monitor. Stock EVDI1.14.15 completes deferred DRM flips from GRABPIX. A native
guest reproduced an outstanding paused flip and a later 20,288ms blocking
modeset with CRTC/plane flip/commit timeouts. This is a confirmed interaction
between Blent's pause policy and stock EVDI, independent of Cinnamon or AMDGPU.
It is a matching mechanism for T1184, not proof of its historical initiating
request or original capture-control state.

The helper now separates media work from the retained output's native completion
obligation. While inactive, its original event loop performs GRABPIX at most
once per 100ms using the existing registered native buffer. It does not request
new updates, normalize pixels, convert NV12, publish frames, allocate new buffers
or reconnect the monitor. A signal cutoff, invalid/failed mode, missing buffer
registration or active media work prevents this paused service. The same
event-loop-owned fallback timestamp paces service and is reset at each pause.

Resume obtains and normalizes a full native snapshot before producing media,
including when the preceding backend grabs already consumed all native damage.
This avoids publishing an old canonical RGBA fallback image after resume.
The independent capture-activity contract documents that an inactive media ACK
does not retire the retained output or forbid its necessary backend service.
The schema, consent and connection lifetime remain unchanged.

There is an unavoidable native readback cost while paused with this stock API.
The 100ms interval limits that service to 10Hz; media conversion, encoding and
publication still pause. It is a service cadence, not a hard real-time bound
under arbitrary scheduling, driver stalls or resource exhaustion.

## Permanent red-before/green-after regressions

Both tests are part of the normal Cargo `evdi_helper` target:

```sh
cargo test --offline --locked -p blent --test evdi_helper t1185_
```

- `t1185_paused_monitor_completes_native_flips_without_media_work` runs the real
  capture loop against an independent deferred-event obligation. A 25ms clock
  model explores the 0–250ms paused interval and stop cutoff; exact token42
  must settle while media stays paused. It checks 100ms pacing, no conversion,
  no new update requests, no disconnect and byte-identical contents of a held
  96-byte NV12 writer lease. A second idle grab must not overwrite completion
  evidence or settle the event twice.
- `t1185_resume_normalizes_the_latest_native_snapshot` uses real allocated
  native/canonical buffers for an 8x8 AB24 mode with padded stride. It checks every visible byte and untouched row padding, verifying
  that native pixels already updated during pause reach the resumed image
  even when the next stock grab reports no new damage.

Both failed on the original production source before behavior changes; the
final unchanged regression source was also tested against an isolated HEAD
baseline. `progress-final-red.log.gz` and `resume-final-red.log.gz` retain the
specific behavioral assertions. The initial model incorrectly replaced its
settled token with zero on a later idle grab; it was corrected to retain
exactly-once settlement evidence and rerun against that original baseline.
That intermediate model failure is not used as repair evidence. The final
regressions pass unchanged against the repaired implementation.

Existing pause, malformed-control, native-buffer/descriptor ownership, allocation
failure, mode, cursor, bounded fuzz, watchdog and race regressions were retained.
No existing test was deleted, skipped or weakened.

## Actual stock-driver guest regression

`host/tests/display_progress_native.c` is a permanent guest-only native fixture.
It uses the real helper, bundled unmodified libevdi and actual libdrm APIs. It
requires the explicit `blent-evdi-review=1` guest command-line marker before
opening DRM. Its expected contract is prompt completion while media remains
paused, with no resume or reconnect needed to settle the original flip.

The same native fixture fails against the original deployed helper:
`T1185 paused retained monitor must complete original flip`. It passes with the
patched helper, checking the original feedback identities, no duplicate event,
subsequent modeset and DPMS off/on while still paused, then resume and cleanup.

Observed values in `native-green.log.gz`:

- Active flip: **16ms**.
- Paused flip: **100ms**.
- Blocking paused modeset: **1ms**, previously **20,288ms**.
- Paused output DPMS off/on: **99ms / 100ms**.
- No `flip_done timed out` or `commit wait timed out`.
- Original helper PID91 reaped as PID91 with status0, original DRM owner closed,
  guest output removed, EVDI unloaded, guest powered off and QEMU exited0.

`native-red-command.json`, `native-green-command.json` and
`native-input-sha256.json` retain commands and source/binary identities. Build
the fixture with installed tools:

```sh
cc -std=c11 -O1 -g -Wall -Wextra -Werror -I/usr/include/libdrm \
  host/tests/display_progress_native.c -ldrm -o native-regression
```

The initramfs, kernel, module and complete raw counters remain local under
`/backups/disk2/blent-work/2026-10-20-t1185/`. The guest has 1GiB RAM and two
vCPUs, no network, filesystem sharing, GPU, USB or host storage passthrough.
It uses cached kernel7.0.0-34 and packaged EVDI1.14.15. The incident uses -38;
its protected kernel image could not be read without administrator access.
During isolated verification, no current driver, desktop setting, live output or
installed helper was changed. No download, package installation, upload, fetch or
push was performed. The later approved helper activation is recorded below.

## Verification and ownership limits

The locked offline normal native collection passes **82 tests** across
`capture_activity`, `conversion`, `evdi_helper`, `evdi_modules` and
`frame_retirement`. `native-suite.log.gz` retains the full run. Existing
ASan/UBSan and race-enabled cases ran with the usual normal-suite flags, while
the existing coverage compiler wrapper retained GCC counters.

`function-coverage.json.gz` measures all seven changed production functions
individually: six at **100%**, `resume_capture` at **12/13 lines (92.31%)**.
All 234 production functions present in the collected native reports exceed
80%; that measured inventory is not a replacement for a whole-project/platform
coverage claim. GCC/gcov version is 13.3.0. The original source fingerprints and
compiler commands accompany the raw reports. One intentionally unused temporary
fixture binary from the existing binary-ownership test has no runtime counter;
its zero report is retained and does not replace counters from actual execution.

`complexity.json` records every new/changed production and test function. Local
authored-source branch counts use compiler function boundaries and the documented
C decision/operator rules; changed production scores are **1–7**, all reviewed
functions are <=9. Clang18.1.3 validates C syntax and rustfmt validates the Rust
wrappers. These are local estimates, not native SonarQube evidence. T1019 retains
the unavailable native measurement; no analyzer was downloaded or installed.

The production fix adds no heap allocation, worker, callback, mapping or retained
identity. The event loop owns admission and the registered native buffer through
each synchronous GRABPIX return. NV12/shared frame leases remain separately
owned by their original writer/encoder; paused native service neither accesses
their pixels nor manufactures a retirement ACK. Existing mode retirement,
control rejection, partial startup, stop and process cleanup retain ownership.
A blocked native operation remains owned by the original helper/process until
actual retirement; this fix does not turn signal delivery into a native fence.
The tests release the original writer lease after loop return and clear callback
hooks before stack-owned fixtures end. The native fixture keeps three immutable
640x480 dumb buffers until the original helper has retired and the CRTC is
disabled; no buffer is recycled during either observation.

These hosted sanitizer results and bounded native traces do not prove every
interleaving, kernel leak freedom or AMDGPU imported-buffer safety. There is no
native KASAN/kmemleak claim. T1184 retains incident-specific attribution and
physical Cinnamon/AMDGPU/-38 acceptance, and the distinct T748/T749/T750 findings
retain their existing dispositions.

## Prepared local build

The verified helper and unchanged bundled libevdi are staged together at
`target/t1185/evdi_helper` and `target/t1185/libevdi.so.1`. `ldd` confirms the
helper resolves that sibling library. `manifest.json` records their hashes and
the permanent regression source hashes. At the build checkpoint the running
installed helper remained unchanged. The owner subsequently approved installing
this helper and restarting Blent; activation evidence follows.


## Approved physical activation

Owner instruction: “Install and restart now”. Installation and restart completed
on 2026-10-20 at **10:51:44 CEST** on Cinnamon/AMDGPU/kernel7.0.0-38-generic.
Only the helper executable was replaced. The stock driver and bundled libevdi
were unchanged. The installed and running `/proc/59391/exe` SHA256 is
`9c4b48c66274d1936f0426820c4be883f47ca25b3645289999beca3045326475`, matching
`manifest.json`. The old executable is retained for rollback at
`/backups/disk2/blent-work/2026-10-20-t1185/deployment/evdi_helper.before`
(SHA256 `260b1a5d7efc08c9b2897eb70482915f153e6a748e0fd28915b76061530cb9ff`).

The backup was written and synced before an atomic replacement in the installed
directory. The running old executable inode was never overwritten. Systemd
restart returned success without reaching its stop deadline. The original host PID 2861,
helper PID 3189 and encoder PID 3524 disappeared; the host reported both child exits cleanly,
and the kernel recorded original helper PID 3189 disconnect and close before helper PID 59391
opened the device. The new host PID 59224 and helper PID 59391 belong to `blent.service`.
This establishes process/file retirement for this restart, not unrestricted proof
of every kernel allocation or callback lifetime.

The bounded observation ends at **10:52:57 CEST**. Twelve completed five-second
capture/encoder/render-ACK intervals show grab totals increasing from 267 to 3145,
211–296 encoded access units and 213–296 render ACKs per interval. The same Cinnamon
process PID 2740 remains running. Read-only X11 queries return with physical HDMI output
and virtual DVI-I-2-1 active at 1280x800@60. No EVDI flip/commit timeout, GPU reset,
BUG, Oops or kernel call trace matches the retained kernel log during this window.

Existing warning paths remain visible: one AMDGPU imported-object mmap rejection,
32 ignored DDC/CI messages, KWin-unavailable warnings under Cinnamon, startup codec
configuration wait, and X11 mapping retry exhaustion. Touch and pointer mapping
both succeeded; Xinput lists Blent Pen as a keyboard, consistent with declined
pen mapping work T592. These logs do not establish new regressions or resolution
of T748/T749. Shutdown FFmpeg pipe errors accompany its clean signal 15 exit.

`activation.json`, `activation-service.log.gz`, `activation-kernel.log.gz` and
`activation-xrandr.txt.gz` retain the identities, hashes, counters and local logs.
The approved restart did not include forced host display-off, lock or suspend.
T1184 therefore retains physical pause/resume/wake acceptance and historical
attribution limits. T914's broader acceptance window remains unapproved.
