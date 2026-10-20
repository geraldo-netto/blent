# T1230: offer explicit lower desktop stream presets

The existing native stream-scale contract supports divisors1–4, but the GUI
offered only Full/Half. New180p and360p buttons select a1280×720 manual desktop
and divisor4 or2. Output geometry is320×180 or640×360. Capture keeps720p geometry;
input mapping, native timing bounds and native backend implementations do not
change. FPS/bitrate/quality remain the user's selections. Visible text explains
the720p desktop and required restart; no preset silently restarts display sharing.
The Half label no longer promises unmeasured lowest latency.

The permanent rendered-UI regression failed first because180p was absent.
It now selects both presets, checks output geometry against independent literal
dimensions, validates/sanitizes, preserves FPS/bitrate and saves without starting
display sharing. All116 GUI tests and the real private-Xvfb accessibility startup
test pass. LLVM counters individually measure the preset function at100% and enclosing settings method at95%.
Raw LCOV, exact executed object/source hashes and red/green logs are retained.
Rustc1.98.1/cargo-llvm-cov0.8.7. Source complexity estimates4/6, not native Sonar;
SonarQube unavailable, no tool download. The startup test stubs its update probe.

These controls borrow one GUI-owned FileConfig for the synchronous frame. Fixed
choices create no native resource, retained callback, queue or additional state
owner. Existing Save and explicit display restart retain their own lifetimes.
Native resources are untouched until the user applies/restarts sharing. No new
backend support or physical compositor qualification is claimed by a GUI test.
Existing scale/native encoder contracts remain authoritative; this change does
not relax EDID or capture resource bounds. Physical desktop360p and180p observations now pass on the selected tablet.
The native encoder reports640×360/divisor2 and320×180/divisor4 from a1280×720
desktop. Android returns real render acknowledgements for both; raw scoped
journal records are retained. The independent180p camera continues producing
frames and the original speaker helper stays alive through display restarts.
Exact original user configuration is restored and normal display sharing
restarted afterward. No display backend or timing-limit implementation changed.
