# T1184: previous-boot desktop hang

The reproduced pause/flip failure is repaired under T1185; see the
[implementation and permanent red/green evidence](../2026-10-20-t1185-fix/README.md).
This report retains the incident and pre-fix diagnostic observations.
Incident-specific attribution and physical target acceptance remain open.

Host-read-only investigation requested by the maintainer on 2026-10-20. Times below
are Europe/Rome (CEST, UTC+02:00). No live display, service, driver, package or
power settings were changed. No network transfer. The later isolated guest
checks below deliberately reproduce the suspected mechanism; no host failure
was deliberately triggered.

## Evidence

Boot `15ccc64c653a49c2aa7c03157777344a` ran from October 19 14:05:02 through
October 20 08:50:44. This is journal boot `-1` at collection. The journal's
next-boot wall-clock start overlaps the old boot's end; comparisons here stay
within the explicitly identified previous boot.

- `kernel.log.gz`: complete previous-boot kernel journal, `short-iso` timestamps.
- `incident.log.gz`: all journal entries from 08:44:00 through 08:51:00 in that boot.
- `Xorg.0.log.old.gz`: preserved previous X server log, originally
  `/var/log/Xorg.0.log.old`; its header identifies the October 19 server startup.
- `xsession-errors.old.gz`: preserved previous session stderr, originally
  `/home/netto/.xsession-errors.old`.

Kernel identifies itself as `7.0.0-38-generic`; its boot messages identify
EVDI `1.14.15`, atomic mode enabled, with two virtual devices. Xorg reports
server 1.21.1.11 and package `2:21.1.12-1ubuntu1.8` in its header.
Use boot messages for the incident driver identity: current on-disk module
metadata is not proof of what the old boot loaded.

## Observed sequence

1. October 19 14:06:03: EVDI card0 connects to `evdi_helper` PID3154.
   The incident does not start with a newly launched helper or a disconnect.
2. October 20 08:46:44: connected/EDID messages and virtual display power off.
3. 08:46:45: power on and unchanged 1280x800@60 mode; helper retains its buffer.
   Kernel reports `evdi_gem_mmap:490 Don't allow imported objects to be mapped: owner: amdgpu`.
   AMDGPU display IRQ work also exceeds the workqueue CPU warning threshold.
4. 08:46:46.864: Blent PID2824 reports
   `original X11 power observer lost error=deadline has elapsed`.
5. 08:46:55: first EVDI `flip_done timed out` and
   `[CRTC:38:crtc-0] commit wait timed out`. Repeated timeouts continue through
   08:50:25: 19 flip timeouts, 11 CRTC commit timeouts and 8 plane commit timeouts.
   AMDGPU resume/reinitialization messages recur between them. These messages
   alone do not establish explicit GPU-reset recovery.
6. 08:47:37: session log reports Cinnamon power daemon timeout setting
   `PowerSaveMode` on `org.cinnamon.Muffin.DisplayConfig`. VS Code's GPU process
   exits at 08:47:37, 08:48:37 and 08:49:36. At 08:47:58 Cinnamon's powertoys
   extension reports a ddcutil timeout; DDC/CI warnings appear in the journal.
7. 08:50:26: Cinnamon logs access to an already disposed `MetaInputDeviceX11`
   from `virtualKeyboard.js:714/730/735`, after several minutes of display stalls.
   This late symptom does not identify the initial failure or prove Blent
   caused an input lifetime defect.
8. 08:50:32: reboot begins and Cinnamon receives SIGTERM. At 08:50:42 systemd
   kills Xorg with SIGKILL; at 08:50:43 Blent stops and its original helper
   disconnects/exits cleanly. Those shutdown signals are not evidence of an
   earlier spontaneous Cinnamon or Xorg crash.

The complete previous-boot kernel log contains nine imported-object mmap
rejections, including October 19 14:06:03 and 20:54:43, hours before this
incident. The rejection alone therefore does not identify its trigger.
All 19 flip timeouts occur during this incident. No incident-window kernel
panic, BUG, OOM kill, segfault, coredump
message, GPU-ring timeout or explicit GPU-reset message was found. Absence
of a log entry cannot exclude an unlogged failure.

## Interpretation and next evidence

The failure localizes strongly to the EVDI/AMDGPU display path during an
off/on transition: virtual-output commits fail to complete while X11 consumers
and Cinnamon power control lose responsiveness. It is consistent with a
desktop hang rather than an independently demonstrated Cinnamon process crash.

The incident mmap rejection precedes the first flip timeout, but earlier
occurrences without flip timeouts weaken it as a standalone causal signature.
Temporal order does not prove it prevented completion. Logs do not identify the originating
power/modeset request, outstanding page-flip event or exact native wait.
Blent was connected throughout; that establishes involvement of its output,
not whether Blent, Xorg/Muffin, EVDI or AMDGPU introduced the defect.

The follow-up below inspects cached upstream driver implementation and existing
isolated fixtures for imported-buffer handling and atomic/vblank completion.
T1185 now owns the isolated pause/completion contract reproduction needed to
narrow T1184 attribution. Establish an independent contract and reproducer before changing
behavior. Any repair requires a permanent failing-before/passing-after test,
event/buffer lifetime and original retirement checks, per-function coverage
and native validation. Live driver changes or display interruption require
separate authorization. T222, T700, T746 and T748 retain their existing scope;
these logs do not resolve or identify those historical causes.

## Follow-up before guest tests: local completion-path inspection

The maintainer describes Cinnamon becoming stuck without an explicit crash.
The retained incident supports that description. In a follow-up, the maintainer
confirms that Ctrl+Alt+F1 reaches a Linux text console and normal login works,
but the graphical desktop cannot be restored. The shortcut was initially
reported as Alt+F1 and then corrected to Ctrl+Alt+F1. This rules out a complete
system lockup at the time of console access, while leaving a kernel display-driver
stall possible. Whether stopping Blent would have restored the session is not
established. No recovery experiment was performed.

Offline source inspection used the existing local upstream tag `v1.14.15` in
`/backups/disk2/projects/evdi`, commit
`3dafd623f5c59ce6fe53f0662107d3e88f868de3`. This matches the logged upstream
version and warning line numbers, but does not establish that the distro build
has identical sources or configuration. The separate prepared `dkms-source`
directory is labelled `1.15.1+plane-fix1`; it is not incident-matched evidence.
No checkout, external repository edit or network transfer was needed.

The upstream 1.14.15 source shows these completion boundaries:

- `module/evdi_modeset.c:66–95`: `evdi_crtc_atomic_flush` hands the CRTC event
  to `evdi_painter_set_vblank` and clears `crtc_state->event`.
- `module/evdi_painter.c:710–733`: under the painter lock, an older retained
  event is completed; a connected painter with dirty pixels retains the new
  CRTC/event pair instead of completing it immediately.
- `module/evdi_painter.c:1053–1188`: GRABPIX takes a framebuffer reference and
  transfers the retained CRTC/event pair to local ownership before mapping and
  copying pixels. Imported buffers enter `dma_buf_begin_cpu_access` before the
  common completion path sends the event and drops the framebuffer reference.
  Returning errors through that common path still sends the event; a blocked
  operation before reaching it is a different condition from an error return.
- `module/evdi_gem.c:488–492`: the logged imported-object mmap rejection is a
  separate `-EINVAL` path. Its presence does not prove GRABPIX failed or that
  this rejection lost a flip event.

These are mechanisms to investigate, not confirmed defects or evidence of an
incident buffer leak/use-after-free. They explain how a live userspace capture
client can participate in deferred display completion. They do not establish
which component first stopped making progress.

Current Blent source also warrants a joined lifecycle check: inactive capture
returns before `evdi_grab_pixels` in `host/evdi/capture.c::grab_now`, and
`on_update_ready` ignores readiness while inactive. The X11 observer timeout
publishes lost display evidence in `host/src/activity/dpms.rs::Probe::run`.
The incident has no capture-control request/ACK trace proving the helper was
inactive, and its deployed binary was not matched to current sources. This is
an unproven interaction to test, not attribution to Blent's pause handling.
Helper DPMS callbacks continue at 08:47:06, 08:47:26 and 08:49:42, so the logs
also do not support claiming its event loop was blocked continuously throughout
the incident.

Next causal evidence must distinguish an event retained by the painter, an event
transferred to an in-flight GRABPIX operation, an inactive helper, and an AMDGPU
buffer/fence wait. Preserve exact event/framebuffer identities across handoff,
power transition, error, disconnect and cleanup; completion must occur exactly
once before original resources are released or reused. Existing isolated
`drm_evdi_vblank_delivery` coverage in the local newer EVDI test worktree checks
one deferred event delivered on replacement. It does not cover this joined
DPMS/imported-buffer/inactive-capture condition, and was not rerun here.
An isolated permanent regression and incident-matched/native evidence remain
required before selecting a behavioral fix. No production/test code changed;
no new coverage, complexity or native-safety pass is claimed.

## T1184 recheck before guest tests: pause/completion conflict (T1185)

An additional read-only journal query preserved the helper's earlier power-off
and later wake in `helper-power-transition.log.gz` (49 lines). The exact query
uses the original boot identity rather than the relative boot number:

```sh
journalctl -b 15ccc64c653a49c2aa7c03157777344a _PID=3154 \
  --since '2026-10-20 02:43:30' --until '2026-10-20 08:51:00' \
  -o short-iso --no-pager
```

Helper statistics show about five grabs per second through 02:44:19, followed by
DPMS OFF at 02:44:22. No further periodic grab-stat records appear through
shutdown, including the wake/hang period when mode and DPMS callbacks are still
logged. Current `capture_tick` returns before capture recovery and statistics
when capture is inactive. These observations support the pause hypothesis;
absence of statistics does not establish the historical control state, exact
grab count or native wait.

Current source provides a concrete scenario for independent validation:

1. A dirty connected EVDI output retains an atomic flip event.
2. The helper is paused, so ready callbacks and periodic recovery perform no
   pixel grabs. Its display connection and framebuffer remain present.
3. That event receives no grab-driven completion. A later commit or disconnect
   can settle a retained event, but neither is proof that the original commit
   progresses promptly. A resume-dependent completion can also be unavailable
   if display state remains unknown after an X11 observer timeout.

This is a source-level cross-component conflict to validate, not a reproduced
kernel failure or proof that it caused this incident. The independently defined
DRM contract is present in the installed matching-kernel header
`/usr/src/linux-hwe-7.0-headers-7.0.0-38/include/drm/drm_atomic.h:91–103`:
`drm_crtc_commit.flip_done` is completed implicitly by sending the commit's
vblank event. The header is contract evidence, not an incident stack trace.

The retained-monitor requirement remains explicit in the October 12 owner
decision in `docs/reviews/2026-10-10-host-power-idle.md:178`: host lock/display-off
keeps the monitor in the desktop layout while capture/encoding pauses. The
implemented T1021 policy and `host/tests/capture_pause.c::t1021_idle` additionally
stop all paused pixel grabs. Do not silently relax either contract, detach the
output, weaken existing tests or issue a premature event that permits buffer
reuse before real native access is fenced. T1185 records the isolated
completion/ownership validation; its result feeds T1184 incident attribution.

Four focused existing automated tests passed during this recheck:

- `cargo test --offline --locked -p blent --test evdi_helper
  t1021_pause_retains_monitor_and_waits_for_original_writer -- --exact`:
  one Rust test drives six C cases for wire/control/pause/raw/boundary/descriptor
  handling under ASan/UBSan. It confirms no paused grabs with a fake EVDI device;
  that fake does not own a deferred kernel flip event.
- `policy-test.log.gz`: supported unknown display state disables display demand.
- `observer-test.log.gz`: stalled original observer clears only display state.
- `supervisor-test.log.gz`: supported source loss/failure/stop cannot restart
  original work; its helper is a private synthetic process, not live EVDI.

An initial policy invocation used an unqualified test name with `--exact` and
selected zero tests; the corrected fully qualified invocation in
`policy-test.log.gz` ran one test. The zero-selection run is not passing evidence.
Each of the four actual selections ran one Rust test with no failures, ignored
tests or skips. Cargo is 1.98.1; GCC is 13.3.0. Source SHA-256 identities are in
`reviewed-source-sha256.json`, based on Blent HEAD
`9b477a5870080c2c587406614a2abf5b0fb6bed7` with the pre-existing uncommitted
T1184 ledger/evidence work retained. No production/test source was changed.
There is no new per-function coverage/complexity claim or native acceptance.

The currently running helper resolves to
`/home/netto/.local/share/blent/appimage/Blent.AppDir/usr/bin/evdi_helper`, with
build ID `a35b0df508ea4b79a1d0c7660bc02703de63f9e5` and SHA-256
`260b1a5d7efc08c9b2897eb70482915f153e6a748e0fd28915b76061530cb9ff`.
Its on-disk mtime is October 19 13:47 CEST. This is a present-binary observation;
it does not prove the previous boot executed identical bytes. The repository's
`host/evdi/evdi_helper` has an older October 9 mtime and is not used as deployed
source-match evidence. No running process was attached, stopped or modified.

## Native T1185 check: deferred flips and matching timeout signatures

The following guest results establish the failure mechanism that the preceding
sections could only hypothesize. They do not establish the historical incident's
exact initiating request or original capture-control state.

`pause-flip-probe.c` runs the unmodified deployed helper and its bundled libevdi
against packaged EVDI **1.14.15** inside a disposable QEMU/KVM guest. The cached
guest kernel is **7.0.0-34-generic**, rather than the incident's **-38**. A
noninteractive read of the protected -38 kernel was unavailable (`sudo: a password
is required`); no password, permission change or replacement was requested.
The -34 kernel and matching packaged module were already local. The VM has
1 GiB RAM, two vCPUs, no network, no host filesystem share and no GPU/USB/storage
passthrough. No Xorg, Cinnamon, AMDGPU or imported GPU buffer is needed for this
counterexample.

The probe owns three immutable 640x480 32-bit dumb framebuffers. It starts one
original helper with the real private capture-control socket, activates capture,
sets the initial mode, and checks actual DRM flip feedback. The finite cases
submit two page flips, pause sequence2, and resume sequence3. Event tokens bind
feedback to each original commit; waiting for twelve seconds is an observation
window, not a claim of infinite noncompletion or an outer process kill deadline.
The guest runner has no elapsed-time termination cap.

Observed results, with full logs and parsed values retained:

- `probe-resume.log.gz`: active token1 completes in **17 ms**. After the exact
  pause ACK, token2 has no completion through **12,013 ms**. Resume, without a
  new commit or reconnect, delivers token2 at once; the count is exactly two,
  and a further 100 ms observation sees no duplicate. This confirms that a
  paused real helper can leave the original flip outstanding.
- `probe-modeset.log.gz`: active token1 completes in **16 ms**; paused token2
  has no event through **12,012 ms**. A subsequent blocking modeset takes
  **20,288 ms**, emitting two `flip_done timed out` messages and one each of
  `[CRTC:38:crtc-0] commit wait timed out` and
  `[PLANE:34:plane-0] commit wait timed out`. These match the error classes
  retained from T1184. A successful ioctl return after those waits does not
  mean it completed promptly. Token2 is eventually delivered after the
  modeset/resume sequence; that case does not attribute delivery to resume
  alone. The final default-mode check uses the same final probe source as the
  modeset case and is recorded separately below.

`probe-final-resume.log.gz` repeats the resume-only case using the final probe
source shared by the modeset experiment: active flip **17 ms**, no paused event
through **12,012 ms**, and the same token2 event delivered immediately after
resume. Exact source hashes match both final input manifests.

`probe-results.json` records all three successful runs. The kernel log is
printed again by guest teardown, so counting the full raw log would double-count
timeout messages: the modeset experiment has **two unique flip timeouts**, not
four. `probe-admission-first.log.gz` retains an earlier fixture rejection:
the first page flip raced the initial modeset and returned `EBUSY`. It is not a
counterexample result; the final diagnostic waits for legitimate busy admission
to clear, without replacing the original buffers or imposing a retry runtime cap.

Each successful guest records `pid=91 waited=91 status=0` for the original
helper, closes the DRM owner, removes its virtual output, unloads EVDI and powers
off. The QEMU process then exits0. Both event tokens and all producer buffers
remain within that one guest/process lifetime. Parent wait observations use
`WNOWAIT` until actual retirement to avoid reaping and then signaling a reused
PID. Failed setup uses the registered cleanup sponsor; unexpected inability to
reap is explicitly unsettled and remains contained by the disposable VM.
The kernel continues to own native references until its own teardown; these
observations are not KASAN/kmemleak evidence or unrestricted lifetime safety.

Run identities and commands are in `command.json`, `modeset-command.json`,
`resume-input-sha256.json`, `modeset-input-sha256.json`,
`final-resume-command.json` and `final-resume-input-sha256.json`; the executed modeset
PID1 script is `probe-guest-init.sh`. Full initramfs/binary artifacts remain
local under `/backups/disk2/blent-work/2026-10-20-t1185/`. The early resume case
preceded addition of the optional modeset diagnostic; its separate binary/source
hashes are retained. The final source accepts no argument for the resume case
and `modeset` for the blocking case. It refuses DRM access outside a guest with
the explicit review marker. Build with installed GCC13.3:

```sh
cc -std=c11 -O1 -g -Wall -Wextra -Werror -I/usr/include/libdrm \
  pause-flip-probe.c -ldrm -o pause-flip-probe
```

This diagnostic is supporting evidence, **not a permanent normal-suite
regression or a fix**. At this pre-fix checkpoint, T1185 remained open for a compatible pause/completion
contract and permanent joined regression, with red-before/green-after evidence
before any behavioral repair. Preserve the owner-required desktop layout and
pause behavior; do not silently weaken the existing zero-grab tests or change
the stock driver. T1184 remains open for historical attribution and target
acceptance: this guest does not reproduce the real Cinnamon session, power
transition, AMDGPU imports or incident binary provenance. The mmap rejection is
unnecessary for this reproduced mechanism, but its historical relevance remains
unsettled.

The new diagnostic has eighteen cohesive C functions. Installed Clang18.1.3
parses the source; an AST branch count aligned with the documented local C rules
measures each at **1–5**, with all results in `probe-complexity.json`. This is
not native SonarQube evidence; T1019 retains that missing measurement. The normal
repository syntax checker could not run because its `tree_sitter` dependency is
not installed in either checked Python environment. No tool was installed or
downloaded. Production and permanent test sources remain unchanged; no new
production coverage or native sanitizer pass is claimed.
