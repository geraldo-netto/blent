# T1184: physical off/wake acceptance

**Resolved on 2026-10-20.** The independently reproduced pause/flip completion
failure was repaired under T1185. The repaired helper with T1186 diagnostics now
passes an owner-approved display-off/wake check on the original physical
Cinnamon/AMDGPU/kernel7.0.0-38-generic target. The owner confirms: **“Both working
normally”**, answering whether desktop responsiveness and tablet image updates
returned after the cycle.

This closes the reproduced failure and current target acceptance. It does not
reconstruct the original incident's exact initiating power request, capture-control
sequence or executable identity. Those facts were not retained in the old logs;
current successful behavior cannot prove the entire historical causal chain.
The [original incident report](../2026-10-20-t1184-desktop-hang/README.md) retains
that limit. No claim of immunity from every future display-driver hang is made.

## Authorized window and exact target

Owner approval: “Run acceptance now”, for one diagnostic service restart,
approximately three seconds of display-off, automatic wake attempt, and recovery
verification. No lock, suspend, package installation, driver replacement, input
injection or network transfer ran. The existing power timeouts/settings were
preserved. This narrow window does not authorize T914's broader media campaign.

Boot: `b759a08f-c81b-4ffc-a471-0ac0b0255e7b`, Linux 7.0.0-38-generic, stock EVDI1.14.15.
The helper executable installed and running has SHA256
`42617137d7a73df7d25bf32a1f831a3256a7fbfd1c9cdec9dd115a010b5dac5c`.
The bundled libevdi and stock driver were unchanged. Cinnamon PID 2740 and Xorg
PID 1983 survived the entire test; neither was replaced to manufacture recovery.

At **11:46:30 CEST**, Blent restarted to activate diagnostics. Original host PID 59224
and helper PID 59391 retired before host PID 95251 and helper PID 95421 took over. The service
reports both original FFmpeg and helper exited cleanly; kernel disconnect/close
records confirm the original helper's device teardown. The new helper's exact
process start tick is 777541. Baseline media ACK sequence 1 and active tablet
render ACKs were observed before the power trial.

## Physical transition and outcome

- **11:47:21.577 CEST:** launch `xset dpms force off` as original child PID 96712.
  An independent `xset q` child PID 96722 reports **Monitor is Off**. Kernel and helper
  record DPMS OFF. The helper accepts and ACKs **sequence 2, active=0**.
- While paused, the same helper records **display-progress begin/end sequence 2**;
  the sampled bracket returns in **1,162µs**, including diagnostic overhead.
  The connected output stays present; media pause does not stop native completion
  service. The original FIFO writer retires its partial frame before pause ACK.
- **11:47:24:** an independently scheduled `xset dpms force on` child PID 96748 wakes
  the displays even if the off/query commands had not returned. Kernel and helper
  report DPMS ON; final `xset q` child PID 96751 reports **Monitor is On**.
- **11:47:25:** helper PID 95421 accepts and ACKs **sequence 3, active=1**. Original
  paused encoder PID 95687 is absent; replacement encoder PID 96755 is present. The helper
  keeps its original process/start identity across the complete off/wake cycle.
- Through **11:48:38**, fifteen post-wake capture/render intervals progress.
  Grab totals increase from 1513 to 4017. Each completed render interval has positive
  ACK samples. The current helper executable fingerprint remains unchanged.
- Cinnamon `PowerSaveMode` returns 0 in 5.73ms; its `GetCurrentState` API responds
  in 5.75ms. Read-only `xrandr --current` and `xset q` complete in 5.02/3.41ms.
  The actual compositor API which timed out during the incident is responsive.
- No EVDI flip/commit timeout, kernel BUG/Oops/call trace, matching GPU reset or
  lost original X11 power observer appears in the retained bounded window.
  Owner then confirms both desktop and tablet are working normally.

All four original xset children were waited/reaped with exit 0. The recorded
`observed_retirement_ms` values are upper bounds at the controller's observation
points, not measurements of each command's execution latency. Wake admission
was independent of off/query completion. No command remains running.

## Ownership, regressions and limits

The unchanged running Blent service remains the authoritative owner of the
connected helper, native registered framebuffer, conversion pool and current
encoder. Retaining these live resources is expected ownership, not cleanup
completion. No helper or native buffer replacement occurred during the power
cycle. Pause ACK reports original media-writer quiescence; it does not certify
retirement of the connected DRM output. A sampled native return is not itself
proof of a particular kernel event or unrestricted imported-buffer safety.

The fix already has permanent normal-suite failing-before/passing-after
regressions for paused native flip completion and fresh resume snapshots.
T1186 adds permanent original request/ACK/native diagnostic ordering, all 32
admission combinations and every millisecond in a one-second pacing domain.
The latest offline normal native suite passes 84 tests; all six T1186 changed
production functions measure 100% individual line coverage. The existing stock
DRM guest fixture checks exact original flip identity, no duplicate completion,
bounded modeset/DPMS, helper wait, buffer owner close and module retirement.
See [T1185 evidence](../2026-10-20-t1185-fix/README.md) and
[T1186 evidence](../2026-10-20-t1186-diagnostics/README.md). No production code
changed during this physical acceptance, so no artificial new regression or
coverage run is substituted for those permanent tests.

This physical observation covers one approximately three-second off/wake cycle,
not an overnight soak, suspend, lock, or every AMDGPU import/fence interleaving.
It provides direct native target evidence complementary to hosted sanitizers
and isolated stock-DRM tests. It is not native KASAN/kmemleak or a mathematical
proof of unrestricted leak freedom. T748/T749/T750 and T914 retain their separate
unresolved scopes; warning/import/snapshot work was not silently closed.

`summary.json` retains target/process identities, positive progress counters,
explicit negative findings, observed native child retirement and owner response.
`service.log.gz`, `kernel.log.gz`, `user-session.log.gz` and the compressed power,
X11/compositor query outputs retain the underlying bounded evidence. No executable
live-power test runner is installed or presented as a qualified reusable tool.
