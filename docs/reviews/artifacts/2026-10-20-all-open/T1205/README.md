# T1205: exact current Android broadcast acknowledgement

Camera/audio invitations require exactly one valid Android completion summary with result 1. Native-adapter and malformed-response tests reject result 10, duplicates and invalid UTF-8.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Both permanent native invitation regressions fail before repair: camera and audio
accept Broadcast completed: result=10. The same tests pass after replacing both
substring searches with one focused ADB response parser. It requires valid UTF-8,
exactly one current completion summary, and exact result 1. Non-completion
diagnostic lines and CRLF are allowed. Old text aliases, fallback parsers and
lossy UTF-8 are not accepted. Nonzero native command exit still rejects.

The production probe includes this current camera adapter and parser. Its separate
remote-stop contract remains T1204; this change does not claim that unscoped stop
is safe. No physical ADB/device actions occurred. Native mapping-removal safety
remains blocked under T1090; no exclusive ownership assumption was added.

The normal suite retains 59 reply cases through each real native camera/audio
command adapter, including malformed, duplicate, non-1, CRLF and current success
replies. Independent pure cases cover integers -1024..1024 and every byte
128..255 inserted at each boundary of an otherwise valid ASCII reply. A failing
native audio command cannot grant acceptance or expose its credential. All130
camera/audio/parser cases pass normally and with native instrumentation. Four
probe ownership/actual compilation cases also pass. Formatting and whitespace
checks pass. Exact source hashes, commands, raw red/green logs and LCOV retained.

Native LLVM executable-line coverage: parser 8/8, camera invite24/24, audio
broadcast20/20 (100% individually). Source-matched export includes only the current
library executable, avoiding retained stale objects. Rustc1.98.1,
cargo-llvm-cov0.8.7. Conservative authored-source complexity estimates <=9:
broadcast_accepted<=5, camera invite<=5, audio broadcast<=3, fixture<=2,
cases<=2, each new native test<=2, bounded oracle<=3, UTF-8 oracle<=3. These are
manual estimates, not SonarQube evidence; unavailable native measurement remains
T1019. No test was removed, skipped or weakened.

The parser borrows the command owner's immutable stdout for this synchronous
call, allocates nothing and retains no views/state. Its linear scan is bounded by
the existing command output admission: stdout4MiB/total5MiB defaults, configurable
up to64MiB per stream and128MiB total. Output budgeting, native child/collector
owners, original completion, cancellation and reap sponsors are unchanged. Test
commands consume the complete bounded stdin and exit before their scoped temporary
files are reused; native owners retain completion/reaping. Rejection grants no
new authority, callback, native handle or reuse permission. Hosted tests do not
establish native tablet acceptance or unrestricted lifecycle safety.

</details>
