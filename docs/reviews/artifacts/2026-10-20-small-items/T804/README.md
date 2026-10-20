# T804: bounded update metadata

Release metadata is admitted before String/JSON materialization. The injectable
`HttpReleaseChecks.maxResponseBytes` defaults to256KiB and accepts1 byte through
1MiB. Oversized or invalid declared lengths reject without reading; unknown
lengths read only through the limit plus an Okio upstream segment (at most8KiB).
A declared/actual extent mismatch rejects. Only admitted detached bytes reach the
ordinary OkHttp BOM/charset decoder and JSON/version policy. This is an explicit
byte budget, not a timeout used as a memory bound. Larger metadata is skipped.

The permanent T804 regressions in ReleaseCheckTest first fail on both API27/34:
oversized known/unknown successful responses publish9.0.0, and a truncated declared
body is accepted. Those assertions pass after the fix. The complete release-check
suite also retains deadline, cancellation and stale-owner tests. Additional bounded
cases cover invalid/extreme limits, minimum/maximum allowed capacity, exact256KiB
UTF8 with multibyte text, malformed lengths, empty JSON and UTF16 BOM decoding.
The fixture owns its bounded payload and HTTP dispatcher until the original body
closes and all dispatcher work terminates; no external network request is used.

Original response/source ownership remains in the existing response.use callback.
Rejected and exceptional reads close there. Detached admitted bytes have their own
short-lived in-memory decoder; its close cannot release the original network
source. No buffer view or callback escapes into a replacement request. Cancellation
uses original OkHttp call retirement; existing session generations reject stale
publication. Per-call storage is bounded by the admitted byte budget, one native
buffered segment and linear decoding/JSON allocations. This is not an unrestricted
heap/leak proof or Android hardware result. Existing ownership/lifecycle scopes
and missing native platform evidence remain unchanged.

Offline Gradle/Robolectric passes the full ReleaseCheckTest suite. JaCoCo0.8.15
records100% individual line coverage for every measured method in the changed
HTTP classes and their callbacks. The repository's pinned Kotlin PSI1.9.20 Sonar-rule
model measures70 functions including tests, maximum6; actual native SonarQube
measurement remains unavailable under T1019. Source hashes, original red/final
green logs, test results and method counters are retained beside this report.
No package download, physical tablet action, service restart or push occurred.
