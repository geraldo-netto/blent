# T1246 joined fixture clock has no elapsed-work cutoff

`ConnectionJoinedCameraFixture.clock` unconditionally threw once its monotonic
elapsed time reached10000ms. A progressing joined actor could therefore fail
solely because its fixture lived longer than ten seconds, conflicting with the
owner's progress-over-runtime-cap requirement.

The permanent normal-suite regression moves this fixture's original monotonic
epoch twenty seconds into the past, then checks that two clock reads remain
available and nondecreasing without opening media endpoints. Both API27 and34
cases fail before repair with `IllegalStateException`; `red.xml.gz` and
`red.log.gz` retain that evidence. Removing only the fixture's elapsed assertion
makes both cases pass. The joined camera fixture/protocol/Binder cohort passes
all12 cases with zero failures, errors or skips; green XML/logs are retained.
Connection consent, I/O and cleanup deadlines are unchanged.

This changes test code only. The clock computes elapsed milliseconds within its
containing fixture lifetime and allocates or transfers no native resource. The
regression owns one unstarted Robolectric fixture and destroys its service in
`finally`; no actual worker, socket, device or process is admitted. Its reflected
epoch is private test setup, not a production clock or retirement witness.

Cached Kotlin PSI1.9.20 measures the changed fixture and test files at maximum3
source cyclomatic complexity; these are estimates, not native SonarQube evidence.
T1244 retains the unavailable native analyzer. Production function coverage is
unchanged because no production function changes.

All Gradle checks use cached providers, `--offline --no-daemon`, explicit existing
Gradle cache and a user/network namespace with loopback only. No physical check,
installed-service change, download, upload or push occurs. Original physical
acceptance and the paused T1238 investigation remain unfinished.
