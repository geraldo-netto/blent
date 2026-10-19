# T1113: isolated writer uses the current frame contract

The permanent normal-suite compile test first fails for both 200/500 ms candidate
builds: publication omits the required original generation and the positional
writer initializer omits idle_control. The fixed benchmark uses the public
begin/target/publish contract and explicit writer settings. Its composition root
includes the required private storage layout directly, without an unusable opaque
fallback. Changed C uses Allman braces (clang-format 18.1.3).

Twenty-three focused/architecture tests pass. The bounded native fixture checks
all 256 byte markers in a 2x2 NV12 frame, reader withdrawal between begin and
conversion, rejection without demand, and original lease retention across the
one-second retirement observation deadline. It then releases that exact lease,
observes quiescence and frees the allocations. A deliberate faulty publication
using the current generation instead of the original is rejected by the same
withdrawal oracle. The correct source was restored before the final suite.

The same AddressSanitizer/UndefinedBehaviorSanitizer/LeakSanitizer fixture runs
the actual synthetic writer for its defined six-second sample, sending pixels
to /dev/null. The capture owner retains storage, writer context, FIFO and stop
flag until pthread_join confirms actual original thread exit; only then are
FIFO, frame buffers, condition and mutex released. This covers normal native
retirement, while unchanged production quarantine tests retain mid-write/fault
coverage. No borrowed target survives publish/resize; leases are accessed only
before their matching release. Rejected demand admits no conversion borrow.
No resize runs concurrently with conversion or retained writer leases.

The first sandboxed sanitizer attempt cannot complete LeakSanitizer because the
sandbox uses ptrace. The identical checks pass outside that restriction; no
sanitizer was disabled. GCC 13.3.0 compiles the native fixtures. Scoped estimates
cover 15 C/Python functions, maximum 7; the C count includes logical assertion
operands. These are source estimates, not native SonarQube evidence (T1019).
This benchmark is coverage-exempt; no production application/helper code changed.
The finite fixture verifies selected borrowed-buffer/generation boundaries and
normal actual thread exit; it is not unrestricted hardware or leak-freedom proof.
Eventual cleanup assumes scheduling and completion of these bounded trusted
children, sponsored through original command joins by T1108's existing helper.
No physical EVDI, tablet, battery, suspend, network or installed-app action ran.
