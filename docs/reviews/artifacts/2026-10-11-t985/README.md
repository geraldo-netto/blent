# T985: original audio consent and Activity controls

Android audio controls and observations bind each direction to its original offer and Activity attachment. Saved preferences cannot consent to a new offer, and synthetic tests do not establish physical process separation.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Camera and audio share exact original publication projection, consumer selection
and observation. Typed adapters retain direction/profile metadata and resource
commands. Audio has independent per-direction consent and consumer observations,
explicit per-offer Start/Stop/background controls, existing passive settings and
an exact original foreground Activity attachment. Persisted settings never grant
a new offer. Requested background audio also retains the existing background
preference restriction. MainActivity supplies focused control interfaces.

Permanent normal-suite tests cover both audio directions, all consent booleans,
invalid resources/visibility/foreign tickets, original offer-to-admission identity,
replacement peers and metadata, sibling events, source withdrawal, delayed actual
cleanup, local Start/Stop/background controls, stale rendered callbacks, fresh
permission on reattachment and original SDK denial. Existing camera and legacy
audio tests remain. Native cancellation is idempotent; the new media fixture
counts the first cancellation rather than assuming the adapter calls it once.
Replayed metadata in an admitted owner is rejected; the replacement fixture uses
a new owner as required by that protocol contract.

Validation uses cached offline dependencies and synthetic JVM/Robolectric native
APIs on Android API 27 and 34. No downloads, host/tablet installs, live USB/ADB,
real pairing or physical hardware tests. The production USB audio capability
remains disabled pending T978; joined host audio is T979 and physical acceptance
T860. Microphone composition remains T859. No real process separation or acoustic
latency is claimed from these fixtures.

Full suite: 1885 tests, 0 failures, 0 errors, 0 skipped.
All 2,710 maintained methods meet individual 80% executable-line coverage; no
unmeasured methods. Changed authored Kotlin complexity: 574 methods, maximum 8.
Inventory/counters and native XML/exec are retained compressed, with source
attestations and authored-constructor counters.

</details>
