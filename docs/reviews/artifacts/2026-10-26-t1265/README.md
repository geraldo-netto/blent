# T1265: reject malformed Android release responses

The new shared-corpus and strict-syntax regressions fail before the production
change on API 27 and 34. All ReleaseCheckTest cases pass afterwards, including
byte bounds, cancellation and original response/dispatcher retirement. The
normal suite retains both new methods. No test-only source directories are used.

The HTTP adapter uses the platform's strict streaming JSON reader, consumes one
complete object and requires document exhaustion. Unknown fields are skipped;
non-string tags remain invalid. The existing configured body limit applies before
parsing. The bounded string and reader are callback-local, the reader is closed
by use on success or rejection, and the original response remains response.use
owned. No new asynchronous owner or native allocation is introduced.

Offline Gradle ran :app:testDebugUnitTest --tests com.blent.ReleaseCheckTest
:app:profileCoverage with scripts/coverage/android.init.gradle and the existing
cached Robolectric offline init script. JaCoCo 0.8.15 measures 100% executable
lines for each of newer, releaseTag and stringValue. Kotlin PSI 1.9.20/local
cyclomatic measurements are retained; all methods are <=9. Native physical
Android acceptance is not claimed by these hosted API 27/34 executions.
