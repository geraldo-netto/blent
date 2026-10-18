# T1059: independent display wake values

The [independent policy](../../../display-wake-policy.md) defines desired CPU,
Wi-Fi and window wake behavior for Waiting, Unsupported, Active, Paused,
Unavailable and Background states. Normal/battery and conservative unknown-route
behavior remain explicit. Display pause/source loss/background remove display wake
demand immediately; they neither revoke other media nor prove decoder retirement.
Pure policy contains no Android/framework types. Intent conversion lives in its
focused native adapter and retains no input reference or native handle.

These values and their conversion are implemented. Production Activity requests
still use Waiting; original native service ownership/enforcement remains T1060,
Activity/current-source/window integration T1061, joined acceptance T1058/T1011
and physical host/tablet evidence T914. No native/power-saving claim follows from
an independently tested value contract. Remaining integration was decomposed into
meaningful dependent TODOs before implementation; decomposition is not completion.

The permanent policy table covers all 72 state/energy/activity/transport inputs.
SDK 27/34 tests round-trip every value, preserve absent-field Waiting fallback and
deny demand for unknown/null/wrongly typed state extras, including bounded 4,096-byte
ASCII invalid values. The unconditional-wake control fails three new tests before
the policy fix; red logs/control and passing focused tests are retained. Existing
native power tests preserve normal locks, battery USB release, idle hysteresis,
foreground rejection and destruction cleanup.

Fresh offline ordinary suite: **2,041 tests, zero failures/errors/skips**. All 16
authored production methods in both maintained files independently measure **100%**
native JaCoCo 0.8.15 line coverage using the existing pinned Kotlin PSI 1.9.20
inventory/inline attribution, with no new exclusions. Source/vector hashes, native
class hashes, raw execution/XML counters and individual reports are retained.
Cached source audit covers 26 production/test functions, maximum 6; this is an
estimate, not native SonarQube certification (T1019). Twelve architecture/contract/
portability checks pass. No network transfer, installation, push or live host/tablet
interruption occurred.
