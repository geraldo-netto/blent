# T1040: reject video headers before payload admission

The connection-owned reader validates the generic length before reading a type,
then validates that type's length and negotiated permission before growing its
reusable backing or reading payload bytes. Unknown types, short frames, malformed
liveness lengths and unnegotiated liveness return false directly from `read`.
Diagnostics have one header-validation owner. Dispatch retains defensive rejection;
the liveness version check follows its admitted two-byte body.

Permanent normal-suite `VideoPacketValidationTest` exhausts the 253 unknown type
bytes, tests generic/type-specific boundaries and valid partial-payload EOF, and
spies on input consumption, read requests and backing identity. The original
implementation fails two of its four tests: an unknown maximum-sized packet reads
until EOF, while a malformed liveness length is admitted. The same tests pass
after correction. Existing invalid-packet oracles now assert rejection during
read; valid framing, fragmented EOF, packet sequences, liveness versions and
borrowed backing/growth oracles remain intact.

Final offline Android suite: 2,000 tests, no failures, errors or skips. Native
JaCoCo 0.8.15 counters cover all nine scoped production methods/functions at
>=80% executable lines: dispatch is 83.33%, all others are 100%. The unchanged
repository inventory and native reader keep inline attribution and separate
callbacks. Source and compiled-class SHA-256 fingerprints, raw XML/exec counters,
source inventory, per-method results and completed-case progress are retained.
The 12 architecture/contract/portability tests also pass.

The cached Kotlin PSI 1.9.20 source audit covers all 58 functions in changed
production/test files; maximum cyclomatic score is 8. This is the repository's
Sonar-rule source count, not native SonarQube evidence; T1019 remains unresolved.

The reader exclusively owns its length and payload arrays for one connection.
Its input remains borrowed from the connection/socket owner. Sinks borrow bytes
synchronously until the next read and must copy before retaining them. Valid
payload storage stays bounded by 8 MiB + 1; rejected headers retain the existing
allocation without growth and consume at most five input bytes. No ownership,
callback, native decoder or connection-retirement lifetime changes. The bounded
tests do not prove unrestricted safety or physical Android compatibility.

Reproduce with installed caches, from the repository root:

```sh
android/gradlew -p android --offline --no-daemon --console=plain \
  -I "$PWD/scripts/coverage/android.init.gradle" profileCoverage
```

`check_kotlin.py` expects its retained sources/scope files expanded under
`/tmp/blent-t1040-evidence`, then checks source equality, native report freshness
and every scoped method's counters. `progress.init.gradle` only records completed
test identities; it changes no suite selection, assertions or time limits.
No download, installation, upload, push or physical device change occurred.
