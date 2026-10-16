# T1022: bound helper announcements and observe failed transport

The helper's private framing adapter limits each wire record to 1024 bytes,
including LF/CRLF when present. Both read buffering and accumulated bytes are
bounded. It preserves partial data across canceled reads and poisons itself on
framing, UTF-8 or native I/O failure. Blank lines, CRLF and final unterminated
records match the independent standard Tokio line reader, including a lone CR
at EOF. No dependency was added or downloaded.

The helper owner now observes its stdout task alongside the original native
child. Failed/closed stdout wakes capture cleanup and remains failed across
repeated waits until fresh successful startup. Task completion never stands in
for native retirement: the original child remains owned and its certificate
stays incomplete until actual process/resource cleanup. Existing capture failure
handling performs that retirement. Original stdout publication revocation and
cleanup before replacement remain the separate T1024 prerequisite.

Seven permanent normal-suite regressions exercise bounded record lengths on
both sides of the limit, terminated and unterminated input, every single-byte
mutation against standard line semantics, valid multibyte UTF-8, canceled
partial reads, terminal native I/O failure, invalid stdout with a live original
child, repeated failure observation and normal child-status handling without a
stdout task. They retain bounded-buffer assertions and actual child retirement.
Before the fix, the behavioral tests fail with `T1022 oversized helper line
bypassed framing bound` and `T1022 stdout failure did not wake capture owner`.
A further red regression catches failure becoming unobservable on a second wait;
the same assertions pass with the persistent failure latch.

Fresh locked offline native collection passes 752 host library and 450 binary
tests, with seven pre-existing benchmark ignores unchanged. An independent
installed-SDK collection passes 17 optional encoder helper tests, including all
new regressions. The verified joined native LCOV covers all 25 helper/framing
production functions individually at 80% or better. Framing, handshake and wait
functions reach 100%; drain reaches 94.4%. Configuration-only reports deliberately
retain their individual missing counters; neither is substituted for the joined
result. Separate immutable manifests, native object/source attestations, raw
LCOV and per-function results are retained here.

The SDK uses the installed runtime, host libclang and existing narrow library
links, with network explicitly disabled. Native objects remain in the two fresh
`/tmp/blent-t1022-evidence/coverage-{cli,sdk}/target` directories. Source audit
covers 63 production/test functions, maximum complexity 8, using unchanged
repository Sonar-rule definitions and cached pinned Rust grammar. This is not
native SonarQube evidence; T1019 remains. No permanent regression was removed,
skipped or weakened, and no live display/tablet or installed app changed.

```sh
python3 /tmp/blent-t1022-evidence/scoped_rust.py coverage-cli
PATH=/tmp/blent-t1022-evidence/sdk-tools:$PATH \
  python3 /tmp/blent-t1022-evidence/scoped_sdk.py coverage-sdk
python3 /tmp/blent-t1022-evidence/merge.py
```
