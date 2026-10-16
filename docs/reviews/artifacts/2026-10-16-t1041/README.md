# T1041: original negotiated viewer capability at capture boundary

`LivenessCapability` and `ViewerActivity` define independent read-only observation
contracts. Each original video-queue receiver owns its membership; owner drop
removes both actual subscription and membership under the registry lock. Weak
membership references cannot prolong that owner. Capture reads original grant
sources without duplicating mutable capability state or receiving native authority.
Readiness requires viewers present and every member acknowledged. Legacy/unmanaged
members, unacknowledged or closed sources deny it. Unsupported custom output
compositions default explicitly to no capability/no wake. The existing viewer epoch
retains its initial-zero/new-subscription contract; theoretical exhaustion is T865.

Observers arm structural/source notices synchronously before reading state, so
ACK, replacement and closure cannot be lost before first poll. Source notice
futures retain only original read views; old notices never retain membership or
grant a replacement. Already closed sources wait without spinning. The registry
retains no history of retired receivers, and observation adds no polling task.
Production's existing sixteen-client admission bounds concurrent TCP grant sources.

Permanent tests first reproduce absent original-grant projection in the queue and
actual authenticated TCP path. A further original-source closure regression
reproduced recursive trait dispatch in the new draft adapter; direct original
watch observation fixes dispatch and preserves the arm-before-first-poll contract.
All assertions remain. Cases cover mixed grants/legacy consumers, ACK/closure and
membership wakeups, actual peer loss, 1,000 replacements, retained late notices,
empty/closed idle waits, bounded membership retention and independent epoch views.
Existing FIFO tests keep their behavioral assertions while adopting the owned
receiver contract. No queued frame, native completion or attachment authority is
created by capability observation.

Fresh offline suites pass 794 host-library and 343 shared-contract tests, with
three existing benchmark ignores. Both native CLI FIFO regressions and seven
architecture/portability checks pass. All 35 scoped production functions measure
>=94.74% native executable-line coverage. The first collection exposed two missing
epoch observation paths; permanent contract assertions cover them in the final
collection without excluding any maintained function. Source/object hashes, raw
LCOV and collector attestations are retained.

Source audit: 50 scoped production/test functions, max complexity 9, unchanged
repository Sonar-rule counting, cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2.
Native SonarQube remains unavailable (T1019); source scores are not certification.
No downloads, installation, push, physical monitor or power transitions occurred.
Capture pause/resume policy remains T1015; this commit supplies its viewer input.
