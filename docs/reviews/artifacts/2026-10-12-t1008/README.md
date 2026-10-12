# T1008: bounded original Linux activity observations

Independent common SourceSettings/Observations contracts define native deadlines,
queue capacity and ordered immutable-epoch publication. Linux adapters pin original
D-Bus service owners and initial receive-position fences. Logind supplies explicit
suspend/shutdown/resume and selected-session LockedHint; no transport-loss, Lock
request or idle inference. UID-verified XDG_SESSION_ID is only a fallback after
GetSessionByPID cannot select the native session. Backend loss clears only its own
fields, preserving independent auxiliary/other-backend demand and ownership.

Wayland Cinnamon/Mutter PowerSaveMode is decoded conservatively. Direct X11 power
cannot follow this potentially stale compositor property, so automatic X11/unknown
session selection reports display unsupported pending the separate T1013 native
DPMS adapter. A read-only installed Muffin query returned 0; no display mode or
power-setting method was called. This is native API/fixture evidence, not physical
power/lock acceptance or active daemon lifecycle integration (T1009–T1011/T914).

The initial feature regression fails before Observations exists. A later permanent
invalidation regression is red without original-owner refresh and green unchanged
with it: publish unknown first, query the exact original, revalidate name ownership,
then restore only the fresh field. Fields and original service names never transfer
to replacements. Invalid native messages close that backend conservatively. Body
size is checked before typed decoding (8192 bytes), property collections at 64;
stock zbus framing allocation remains in the external dependency. Queue overflow
and revision exhaustion close publication without wrapping or replaying old Running.

Permanent normal-suite tests cover overlapping sleep/shutdown, lock/display versus
auxiliary independence, owner replacement, stale/foreign/malformed signals, selected
session UID/fallback, unavailable/unsupported sources, invalidation refresh, actual
stop/retirement and a held native task that cannot retire early. Bounded properties
cover 1024 native mode mutations, power-state combinations, all capability masks,
queue/deadline ranges and epoch/revision exhaustion. No tests were removed, skipped
or weakened. Private D-Bus fixtures own their sessions and never mutate the desktop.

Final fresh locked/offline collection passes 725 host and 308 common library tests.
Three pre-existing opt-in benchmark ignores remain. All 63 scoped production
functions pass individual native 80% executable-line coverage; none missing.
All 107 reviewed production/test functions have complexity at most 9; the whole
project gate passes 22,141 functions with none above 9. Sources and the collector
were immutable from snapshot through check; exact commands, ELF/source hashes,
LCOV and per-function reports are retained. The earlier diagnostic counters are
not mixed into final evidence.

No downloads, installation, production service changes, USB/ADB, forced host
suspend/lock/display-off or tablet interruption. No runtime performance/power
benefit is claimed. Unsupported desktop capabilities and physical acceptance
remain explicit, with direct X11 observation tracked under T1013.
