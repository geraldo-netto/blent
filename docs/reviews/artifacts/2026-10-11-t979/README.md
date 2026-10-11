# T979: host audio transport controls and full boundary validation

The portable Audio panel depends on AudioController and independent media target
contracts. Native factory and direction capabilities are explicit boundary inputs.
Linux exposes ADB for microphone/speakers and direct USB for speakers; USB microphone
remains unavailable until T859. Both camera and audio project selectors to one
focused target editor; existing camera behaviors, strings and tests are retained.
Rendering preserves unavailable saved targets. Editing/Apply remain passive.

Permanent red/green regression first failed because a malformed full USB target
reached a permissive backend: only AudioProfile was validated. Start now validates
all AudioOptions and direction/transport availability before native construction.
Bounded lengths 0–128, every invalid byte, UTF-8, ambiguous selectors and buffers
0–240 reject safely. Tests cover unavailable backends, failed construction and
exact valid direction/preferences. Interactive GUI regressions retain camera
selection, speaker full-peer selection, passive persistence, manual Start/Stop,
processing/background choices and display independence.

Fresh offline native GUI suite: 110 unit tests and one private headless startup
fixture pass. All 13 affected production functions meet individual 80% executable-
line coverage; no missing counters or exclusions. Changed-source complexity:
168 functions, maximum nine (the main file contributes unchanged existing methods).
Collection uses immutable inputs and an exclusively owned fresh target. Compressed
LCOV, source/native inventories, manifests, attestation, counters and red/green/full
GUI logs retain evidence. The startup fixture uses private Xvfb/session bus, settings
and command stubs; it does not operate the live host/tablet. Collector forces Cargo
offline, including its nested workspace build. No downloads or installations.

Speaker-only physical acceptance remains T860, explicitly deferred under the
owner's postponed live-test policy; microphone composition/acceptance remains
T859 and SDK-wide native-audio qualification T835. Interface/fixture readiness is
not acoustic or device support qualification. Attestation includes the pre-existing
uncommitted TLS fixture; unrelated edits remain unstaged and are not in this commit.
