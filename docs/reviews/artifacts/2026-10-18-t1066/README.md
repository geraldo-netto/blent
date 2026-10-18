# T1066: concurrent audio fixture command observations

The full offline Android suite completed 2,103 cases with one failure: an audio
retirement callback appended to an ArrayList while the test iterated it. The new
permanent deterministic test holds an original iterator while the callback appends;
it fails before the fix with ConcurrentModificationException and passes afterward.
The fixture now uses CopyOnWriteArrayList for stable observation snapshots. All
three AudioAccessObservationTest cases pass; existing original-owner/native-cleanup
assertions are retained. The fixture owns its bounded command log through close;
this patch adds no coroutine, allocation owner or production synchronization.

Cached pinned Kotlin PSI 1.9.20 audits 34 authored functions/callbacks,
maximum source estimate 5. Native SonarQube remains T1019.
Raw before/after logs, source hashes and faulty control are retained. The combined
focused collection also checks T1060's in-progress timer and power integration;
this commit changes only the audio fixture and its permanent regression.
