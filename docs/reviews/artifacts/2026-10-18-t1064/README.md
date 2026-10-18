# T1064: original observer fixture handoff

T807's Activity fixture overwrote an active power binding. Activity stop then only
retired the replacement, leaving the original foreground observer reachable.
The permanent T1064 assertion fails on SDK 27 and 34 before cleanup. The fixture
now stops the actual original binding, drains its UI cancellation, and requires
its original observer job to have completed before installing the replacement.
Unconditional fixture cleanup also retains that original binding on assertion
failure. Existing T807 session-retirement assertions remain.

All eight retirement-failure cases pass. The complete targeted suite also passes
with the committed binding implementation substituted temporarily, proving this
test-only patch does not depend on T1060's in-progress native integration. The
integration file was restored automatically; versions/hashes and both collections
are recorded. No production function changes or native retirement claim are made.
The original observer's job completion is its coroutine settlement evidence,
separate from any queued/native power obligation still owned by the application.

Cached pinned Kotlin PSI 1.9.20 source audit covers 33 test/helper functions,
maximum 3. This is an estimate, not native SonarQube evidence (T1019). Source hash,
faulty handoff, red and green logs are retained. No network transfer, installation,
push or live device interruption occurred. T1060 remains in progress; T1065 records
the separate shared worker fixture quiescence gap discovered during integration.
