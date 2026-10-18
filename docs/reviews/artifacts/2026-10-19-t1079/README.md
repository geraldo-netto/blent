# T1079: progressing integration commands retain ownership

The normal offline Rust/Android interoperability harness imposed a 300-second
build/test kill cap and a second five-second cleanup cap. These were elapsed
limits, without evidence that compilation had stopped progressing. The command
now waits for its original child while retaining the output log; callers can
monitor completed build stages. Nonzero exits still fail with retained output.
The independent unanswered-approval protocol deadline remains unchanged.

Four permanent command tests fail before this change and pass afterward. Their
bounded process double emits a completed-module observation and rejects an
imposed wait deadline. It also exercises cancellation, an already vanished group
and an unsuccessful exit. These are explicit faulty controls, not a reproduction
of an actual 300-second stalled compiler. The first exploratory invocation also
collected an imported interoperability TestCase: its Android wrapper stopped at
the sandbox's read-only Gradle cache, before Android execution. The import now
uses the module so discovery collects only the four intended command regressions;
the separately named `red-only` log is the definitive failing collection.

Each invocation owns one original Popen handle, one new process group and one
open output file. Normal exit is joined before closing the file. On cancellation
or a wait exception, a still-live original process is signalled and joined before
the exception escapes and the file closes. A vanished-group race still joins the
original child. No process is released merely because a monitoring interval
expired. Child tools remain responsible for their own normal descendant cleanup;
this change does not establish unrestricted native process-tree safety. The
regressions allocate no actual child or clock, and their process/log state ends
with each test.

Python 3.12.3 source audit: 13 functions, maximum complexity estimate 3, using the
repository's PythonScore visitor without importing unavailable native parsers.
Native SonarQube evidence remains T1019. This nonessential test harness has no
new production function or new per-function coverage exemption. No transfer,
installation or physical device operation.

The actual offline Rust/Android TLS interoperability and unanswered-approval
fixtures also pass: six total Python cases including the four command regressions.
The native interop JUnit results are retained alongside the command log.
