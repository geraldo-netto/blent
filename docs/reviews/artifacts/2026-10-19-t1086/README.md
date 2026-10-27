# T1086: bind the original auxiliary transport target

Each manual media intent retains its first successfully resolved tablet target. Fake-ADB and concurrent tests prevent silent reselection; target identity alone grants no media permission or physical support.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The portable OriginalTarget contract owns one manual intent's immutable requested
selector and first successful effective selector. Native lookup and media grants
are outside it. No clone/reset/reselection API exists. Explicit serials and typed
USB identities bypass discovery; automatic ADB resolves before any invitation.
All successful callers borrow the same committed original, including concurrent
or late resolvers. Existing cached intent never queries discovery again. Lookup
errors or cancellation before commitment expose no target; a fresh manual intent
may choose a different tablet. Missing original devices must fail in the native
connection adapter rather than silently falling back. Actual camera/audio wiring
is T1082/T1083, so this prerequisite alone changes no automatic-resume behavior.

OnceLock owns the single committed target. Returned views borrow that owner;
serial preferences are copied at construction and cannot mutate through callers.
Storage is one requested and one effective selector, O(serial length), or typed
fixed-width peer identities. Explicit serial bytes retain existing semantics,
including whitespace and shell metacharacters; OS argv validation remains the
native boundary, including rejection of NUL. Automatic command output preserves
existing trim/UTF-8/empty rules. The pure contract allocates no native resources,
spawns no task, retains no callback and cannot publish a cancelled late result.
Concurrent losing results are dropped normally. Lookup callbacks must sponsor
native resources independently; media authority and current consent/connection
validation remain separate requirements for every attempt.

Linux media_target::bind lazily resolves ADB through the shared native command
adapter. Explicit targets work without an ADB executable. The existing camera
reverse-map adapter, already reused by audio, now calls that same command output
implementation. Native command failures say ADB command failed rather than the
previous camera-specific prefix. Typed arguments, five-second command protocol
deadline, bounded configured output, UTF-8 decoding and mapping ownership are
preserved. No tablet commands were run: tests use a private fake executable.

The native resolver requires a borrowed completion observer. It records the
original child and collector proof before their results can escape. These
observations do not own resources: existing command/collector admissions retain
actual cleanup on rejection, cancellation, timeout, spawn failure and exhaustion.
The observer is called synchronously and is never retained by background work;
the outer workflow must retain and settle these exact proofs before replacement.
No callback future can independently create another command after cancellation.
Conditional cleanup still assumes actual OS completion/evidence and fair retained
polling. Neither a logical result nor a dead PID substitutes for those fences.

Seven permanent normal-suite tests pass. Pure cases cover 16,384 bounded serial
values, empty/error rejection, explicit ADB and USB no-fallback, cancellation,
late lookup completion, both first-winner orders and 16 actual two-thread races.
The isolated native fixture changes discovery A to B: retained A stays selected,
connection fails when A disappears, and only a fresh intent chooses B. It checks
missing executable, command failure, invalid UTF-8, empty output, spawn/NUL
rejection and twelve exact/over output-budget boundaries. Cancelling a live fake
lookup waits for its exact child and collector proofs, then confirms no late A
binding. Native proof storage is bounded to two per command and cleared only on
actual settlement. Fake logs/config/gates belong to one private child and temp
directory; environment changes cannot affect the parent. No grants or devices
are opened. A deliberately redundant-lookup control fails the permanent cached
intent test. Existing camera invitation/mapping and audio rejection cases pass.

Fresh exclusively owned offline LLVM coverage records all six new production
functions at 100% individually; source and native object attestations are retained.
All 12 architecture checks pass. Source estimate: 29 functions including fixtures,
maximum 6, repository counter with pinned cached tree-sitter-rust 0.24.2 and
tree-sitter 0.25.2. Native SonarQube remains unavailable under T1019. Shared
no-default-features compilation is recorded separately. This does not establish
physical ADB/USB/media support or unrestricted native memory safety.

</details>
