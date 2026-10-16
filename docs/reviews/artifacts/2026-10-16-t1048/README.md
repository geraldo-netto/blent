# T1048: optional encoder fixture uses the owned receiver contract

T1041 migrated video receivers to an owner that couples actual subscription and
read-only viewer membership. Its default CLI tests passed, but the optional native
encoder fixture still called the removed `blocking_recv` method. The installed
SDK's test compilation exposed E0599 at `encoder.rs:557`; the retained red log
records that exact failure.

The synchronous FIFO fixture now waits through the existing asynchronous `recv`
contract using a focused test-only current-thread runtime adapter. All native
frame, FIFO path, non-UTF8 path and restart sequence assertions remain unchanged.
No production alias, receiver API expansion or weakened oracle was introduced.
The normal optional encoder suite remains the permanent regression; this is a
test-caller migration, not a production behavior change needing artificial tests.

Nine existing native encoder tests pass with installed Freedesktop SDK 25.08 and
libavcodec 61.19.101/libavutil 59.39.100, offline. Optional capture/worker test
compilation succeeds. Only two test functions change, source-audited at maximum
complexity 2 with repository Sonar-rule counting and cached tree-sitter-rust 0.24.2
and tree-sitter 0.25.2. Native SonarQube remains T1019; no new maintained production
function requires line counters. The T1046 transaction working tree is present
for SDK build verification and is committed separately with its own evidence.
No downloads, install, push or physical media/power transition occurred.
