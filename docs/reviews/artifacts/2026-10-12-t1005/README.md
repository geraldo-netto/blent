# T1005: joined fixture recipient snapshot race

The Binder fixture now iterates a coherent recipient snapshot instead of racing separate size/get calls. Exact original callbacks and unlink semantics remain tested.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The native joined fixture uses CopyOnWriteArrayList for concurrent Binder recipient
link/unlink. Kotlin toList optimizes a one-element list using separate size/get
calls; unlink between those calls throws ArrayIndexOutOfBoundsException. T1004
stress observed that exact stack during cleanup. The fixture now uses the
collection's own forEach snapshot, retaining each callback without an indexed read.

Permanent API 27/34 regression injects an unlink immediately before an indexed
lookup. It fails before the one-line fix (both SDK runs), then passes unchanged.
Additional 0/1/2/32-recipient cases unlink inside callbacks and verify all original
snapshot recipients fire exactly once. Cleanup and simulated death semantics remain.

Fresh ordinary offline suite: 44 tests, no failures/errors/skips, including all
existing joined microphone/speaker and T1000 duplex cases plus the new regressions.
Changed authored fixture complexity: 49 methods, maximum 5. No production method
changed; fixtures are not production native-counter owners. T1000 retains its
historical full native method gate; no fresh whole-project counter claim is made.

The T1004 stress case independently reproduced admitted audio losing visibility
before native acquisition; that production-path investigation remains open. T1005
only fixes the confirmed fixture snapshot race. Its commit leaves ongoing T1004
stress/diagnostic edits and the pre-existing unrelated TLS fixture unstaged.

No downloads, installations, live USB/ADB, services or tablet changes. Tests use
cached Robolectric with controlled native devices and simulated Binder death.

</details>
