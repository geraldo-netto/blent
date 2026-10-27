# T947 native ownership through fatal failures

Fatal cleanup failures retain the original native owner and block replacement while propagating the error. Synthetic Error tests verify ownership paths, not real heap exhaustion or hardware faults.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

ResourceLane always attempts actual native retirement in finally, even when an
execution-ended observer throws Error. Successful native retirement may then
publish proof and admit the successor; the original observer Error still escapes
to its coroutine handler. Fatal native release instead retains the original run
in RetirementGuard, closes admission, publishes no actual-retirement proof and
propagates the original Throwable. Later failures cannot erase the first owner.
A missing proof before the guard was entered also retains the current owner rather
than admitting over unknown native ownership. Unopened pending requests may still
publish safe no-start proof. Other resource lanes remain independent.

All three permanent regression methods are red against the preceding implementation
before the fix and green afterward. Coverage includes throwing observers, synthetic
AssertionError/OutOfMemoryError/LinkageError, direct Throwable, retained first
failure, blocked replacement, exact no-start proof and independent native owners.
The retained diagnostic cause and direct guard boundary preserve original identity.
Coroutine stack recovery may copy propagated Errors; handler assertions verify
class, message and a bounded cause chain containing the exact original object.
This corrects a fixture assumption without weakening any ownership/proof assertion.
Synthetic errors do not exercise actual heap exhaustion or hardware failure.

All 937 offline connection, resource, worker, camera and audio/display tests pass.
All 28 scoped methods in ResourceLane/RetirementGuard meet individual native JaCoCo
80% executable-line coverage; all authored constructors pass. All 25 reviewed
production/new-test functions have cyclomatic complexity at most nine. T942's
permanent 256-step replacement/stop property remains in the normal suite. Native
counter mappings, source hashes/inventory and red/green logs are retained. This is
scoped coverage, not a whole-project or physical hardware qualification.

Production camera remains disabled pending portable capture binding T943, original
connection-attempt adapter T948, Activity/permission integration T944 and joined
composition T932. Physical camera acceptance remains T858. No downloads, installation
or live host/tablet changes.

</details>
