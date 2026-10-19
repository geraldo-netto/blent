# T1121 exact native idle records

Permanent normal-suite regression fails before the fix: a valid five-field
record followed by NUL and more bytes authorizes sparse cadence. The parser
now returns its borrowed end cursor; the validator requires that cursor equal
the original read-count boundary. No bytes after an earlier NUL are ignored.

The test uses the public file-reading API for 500 and 1000 ms records: 16 suffix
lengths x 256 byte values x 2 cadences = 8192 hidden-suffix cases, plus every
truncated prefix and every NUL position within both valid records. Valid records
still pass. Replacing the exact-length comparison with a non-null-only check
is rejected; finally restores the exact source before final measurement.

Final normal suites: 64 helper + 1 separate-module + 1 frame-retirement tests,
four threads, zero failures/ignored, existing ASan/UBSan/LSan/TSan selections.
Outside ptrace sandbox; no device/network actions. All five maintained production
functions in the changed parser module have 100% individually measured native
executable-line coverage. Clang 18.1.3 inventories every source definition;
GCC/gcov 13.3.0 raw counters, notes, commands and source hashes are retained.
An unused compilation's missing gcda warning remains visible; missing counters
are not treated as execution. Architecture checks: 5 pass. Source audit: 76
C/Rust functions, maximum estimate 9; native SonarQube still missing (T1019).

Parser returns only a borrowed cursor within the caller's <=160-byte stack
buffer; no view escapes the synchronous read/validation call. Positive count
below 160 is checked before termination or pointer arithmetic. The five-field
loop rejects signs, overflow and missing separators; cursor comparison rejects
trailing bytes. No new allocations, callbacks, native descriptors or cleanup
paths are introduced. The existing file reader closes its owned descriptor on
every path. This bounded corpus/native tooling does not prove unrestricted
native safety. Replacement-source admission remains separately tracked T1120.
