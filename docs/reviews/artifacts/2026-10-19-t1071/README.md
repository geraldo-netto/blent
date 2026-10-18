# T1071: fresh coverage test compilation

The historical T1069 collection ran an unchanged caller against an obsolete
one-argument StreamingPowerRuntime.open signature after production exposed the
two-argument method with a default permit. Twelve cases failed. Its retained
javap evidence is copied here. A clean source-attested rebuild passed; the exact
incremental compiler invalidation cause remains unidentified. An ordinary small
API-change probe also passes incrementally, so it does not reproduce that cause.

The permanent normal Python suite now includes an offline native Gradle fixture.
It compiles an original caller, changes the API, and deliberately records the old
class as the compiler task's current output. This is a bounded faulty control,
not an attribution of the historical compiler defect. Rerunning unchanged test
sources before the fix reuses UP-TO-DATE compilation and fails the old caller
with NoSuchMethodError: two cases run, one fails. With the coverage init script's
fresh-compilation guard, both callers pass. Unit-test Kotlin tasks disable
incremental compilation and reject up-to-date reuse when that init script is
applied. Normal builds outside coverage collections retain their existing policy.
The original twelve Android power edge cases also pass under the actual guard.

The fixture uses the project's installed AGP8.2.0, Kotlin1.9.20, SDK34 and cached
JUnit4.13.2 through --offline; no package is downloaded. Each command owns one
child/group/log through the shared T1079 command helper. Its temporary project,
saved original class and injection marker remain scoped to the test, and are
removed only after joined command completion or cancellation cleanup. Success
requires all three stages to complete and both native test results to pass
without skips. Production app behavior and API contracts do not change.

Source audit: 16 Kotlin functions/closures, maximum Kotlin PSI1.9.20 estimate1;
three Python functions, maximum Python3.12.3 estimate2. New Groovy configuration
closures have manual estimates at most2; these are not native Sonar measurements,
and T1019 retains the native Groovy measurement gap. No new maintained production
function or coverage exclusion. The prior clean2,234-test Android collection is
historical evidence, not a newly rerun full suite under this guard.
