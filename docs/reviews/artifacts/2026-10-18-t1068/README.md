# T1068: narrow original-source relevance

DisplayActivityView defines read-only current-snapshot relevance independently of
control, UI and native implementations. One pure source owner composes the existing
inbox, carries original history and replaces its original identity. ControlSession
uses that owner and exposes only its read interface. Queued consumers can retain
this small source view without retaining sockets, callbacks or Activity objects.
Relevance has a read linearization point; it is not an asynchronous lease, media
permission, native quiescence or release proof. T1069/T1061 retain power consumption
and Activity/window integration.

Permanent tests reject foreign/equivalent snapshots, old revisions with the same
opaque source, closed observed views and replaced sources; duplicates preserve exact
current relevance. A bounded 64-generation check retains 192 old views and proves
none revives. Control tests verify one read-only owner across original replacement.
The deliberate same-source-only control fails four SDK 27/34 cases and is restored
automatically. Existing original history, malformed-input and decoder retirement
regressions remain. The contract documents containing lifetimes and state bounds;
this patch adds no native allocation, queue, timer or asynchronous cleanup owner.

Final offline ordinary suite: **2,159 tests, zero failures/errors/skips**. All **13**
maintained authored production functions independently measure **100%** native JaCoCo 0.8.15 line coverage. Frozen source/vector and compiled class
hashes, raw counters and pinned Kotlin PSI 1.9.20 inventories are retained, with no
new exclusions. Source audit covers 113 production/test functions, maximum estimate
8; native SonarQube remains T1019. Twelve architecture/contract/portability checks
pass. Two unchanged intentional deprecated permission calls are recorded as T1070.
No network, installation, push or live device operation occurred.
