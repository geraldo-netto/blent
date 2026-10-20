# T1197: native readmission fixture observes actual exit

The original instrumented failure is retained with T1194: second-child retirement
returned Incomplete where the test required Exited within a one-second grace.
Neither that log nor the old test recorded enough timing to identify which
startup/exit/coverage-flush interval exhausted the grace. It does not establish a
production native leak or allow us to declare an Incomplete report incorrect.

A permanent causal regression extracts the original retirement operation unchanged,
then exercises that exact readmission path while an original inherited stdin
socket stays open. In a disposable subprocess, the existing seccomp control
makes kill requests report success or denial without actually killing the child.
The parent holds the sole writer through the original one-second grace plus20ms
reap boundary. Before repair, the operation returns before release and the test
fails. The failure is asserted only after gate EOF and actual original completion;
no child is abandoned to manufacture the counterexample. This proves the fixture
assumption fails under an admissible held/startup schedule, without claiming it
reconstructs the missing historical timing.

The fixture now waits for its actual original child exit before asserting the
Exited retirement phase. This is a readmission/capacity assertion, not a process
startup benchmark. Production retire/control deadlines are unchanged. The original
first-child Incomplete phase, accepted/denied signal errors, capacity rejection
while the child is held, actual completion and process disappearance assertions
are retained. The causal test requires no return before release, then Exited,
original completion and native process disappearance; it does not merely accept
any phase or inflate the old grace. Accepted and denied signal controls both pass.

All393 shared library cases pass normally and with native LLVM instrumentation
(rustc1.98.1/cargo-llvm-cov0.8.7). Raw red/green/full-native logs and source hash
retained. One rejected coverage CLI combination (--no-clean with --no-report)
is retained separately; it executed no tests. The corrected instrumented command
uses --lcov. No production function changed, so no new production coverage claim
is required. Formatting/whitespace checks pass. Manual source complexity estimates:
readmitted_retirement1, readmitted_fixture<=2, assert_readmitted_retirement<=7,
new t1197 regression<=1, changed t814 test<=7. Native Sonar remains T1019.

One child admission and one UnixStream pair are owned per causal stage. The
receiver is transferred into the child's stdin as an inherited owned descriptor;
it is not a ChildStdin that retire can close. The sole writer stays with the
parent and closes on normal release, panic or process loss. EOF permits the child
to finish, flush bounded fixture files and exit. Original reaper admission remains
funded until native observation and destruction. Temporary paths are retained
through completion; no released child identity is reused as stop authority.
The outer command owner waits for the real test subprocess and original proof.
Cleanup assumes child/scheduler/reaper progress after EOF; the held state cannot
settle early from timeout or successful signal alone. This bounded Linux fixture
is not unrestricted process safety or unavailable platform acceptance evidence.
No device/service, Windows/EVDI code or network transfer changed.
