# T1045: shared per-session viewer demand

Per-session viewer continuity now detects a coalesced empty-and-rejoin transition without restarting for continuous membership. Shared selection/capture views preserve saved preferences and original machine fences.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The daemon's global source deliberately publishes `Feed::snapshot(true, false)`:
it cannot own each session's receiver membership. The independent capture context
now carries immutable saved activity preferences from primary/extra ADB/USB
construction. Linux construction creates one read-only display-work view shared
by selector and capture adapter. Selection now observes local `pause_without_viewer`
and original machine activity through that same interpretation. Native capture
consumption remains T1015; native retained-monitor quiescence is T1046.

The original receiver registry owns viewer continuity alongside membership. Its
opaque identity changes after the last receiver leaves, avoiding counters and
retaining coalesced empty/rejoin edges. Late joins and nonfinal departures retain
continuity. Immutable observations and notices keep no receiver/attachment grant
or native authority. Presence is independent of liveness ACKs: legacy viewers
still count, while an ACK without actual membership grants no demand. No-viewer
preference applies even when machine capabilities are unsupported. Explicitly
unsupported presence observation and disabled preferences retain their fallback;
machine stop fences and supported-source closure/loss still deny work.

Five initial independent viewer assertions and the saved-preference assertion
fail before implementation; the same tests pass afterward. Permanent cases cover
empty/legacy/negotiated viewers, coalesced replacement, continuous late joins,
foreign continuity, count boundaries through `usize::MAX`, disabled/unsupported
preferences, first-poll change/closure races, pending/failed/stopped machine
publications, shared source fences and no idle polling. Joined prepared-session
and extra-session construction preserve saved preferences; the actual production
selector calibrates only when allowed and does not restart for continuous joins.
It uses an isolated harmless encoder fixture with no capture/device access.

The source audit covers 79 production/test functions, maximum complexity 8,
using unchanged repository Sonar-rule counting with cached tree-sitter-rust 0.24.2
and tree-sitter 0.25.2. Native SonarQube remains unavailable under T1019. Seven
architecture/portability checks and the normal production CLI compile pass.
No downloads, installation, push or physical power/tablet changes occurred.

Full native suites pass 808 library and 508 daemon tests, with three and four
existing ignores respectively. All 30 scoped production functions meet 80% native
executable-line coverage individually: minimum 83.33%, with 28 at 100%. Raw LCOV,
source hashes, original native object attestations and per-function evidence are
retained. The continuity class-change rejection line remains unhit; it is not
excluded from the count. No native SonarQube certification or physical acceptance
is claimed.

</details>
