# T998: joined Android leased microphone uplink

Production tablet composition now explicitly owns microphone resources and
advertises display/camera/microphone/speakers (15). T996/T997 original consent,
opaque permission replies, native endpoint exposure and consumer proof select the
existing isolated MicrophoneWorkerService and authenticated mono PCM capture path.
Host microphone remains disabled until T999; physical acceptance remains T859.

The permanent production composition case fails against capability 11 because
no microphone offer exists, then passes unchanged after composition. Existing
capability assertions require 15; existing camera and speaker consent/denial/cleanup
assertions remain. No invitation opens native media before explicit local intent
and fresh permission. No fallback or compatibility path is removed.

Eight permanent joined cases on API 27/34 share real private connection/media
services, Binder codecs, accessory admission, actor/presentation, original UI
consent, management/mux and loopback TCP. Controlled synthetic MicrophoneDevice
and process-death notifications establish protocol/ownership, not physical capture,
actual process separation, acoustic AEC or older-device TLS/provider qualification.
Shared audio fixtures retain every speaker regression and avoid duplicate native
service/consumer orchestration. Synthetic read time advances the synthetic native
clock explicitly; no unrelated application or host clock is modified.

Tests verify original SDK grant before preparation, authenticated host grant before
sampling, processing override, buffering/input/gain/background choices, clocked and
unclocked mono samples, increasing native counters, SDK denial/unproven grant,
pending Stop/connection loss and stale results, fresh permission loss, failed
native open, worker death and held noncancellable reads. Display remains responsive
while microphone cleanup is held. Actual native cleanup and private worker-death
proof remain separate; Stop, Ended, EOF and deadlines cannot release admission.
The open-failure fixture waits for stable original lease/proof rather than depending
on transient route visibility after failure. No assertion is weakened.

All builds used cached offline dependencies. No downloads, installations, live
USB/ADB, physical host/tablet or service changes. Automated simultaneous isolation
remains T1000; physical microphone/duplex T859/T846 and native SDK-wide audio T835
remain explicit. Inventory includes the pre-existing uncommitted host TLS fixture;
this commit does not change it.

Final full suite: 1939 tests, 0 failures, 0 errors,
0 skips. All 2,741 maintained methods pass individual native 80%
line coverage. Changed authored complexity: 387 functions, maximum 5. Matching
inventory, native counters, source fingerprints, constructors and logs retained.
