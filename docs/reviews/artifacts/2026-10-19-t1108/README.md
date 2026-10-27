# T1108: original benchmark command ownership

Benchmark commands retain their children through actual completion and repeated cancellation. Offline command checks passed; a separate C replay API drift prevented measurement, and arbitrary daemonizing children remain outside the ownership claim.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Offline compiler and codec analysis use one logged/captured command sponsor.
Compilation/analysis has no elapsed kill cap. Live replay and transfer deadlines
remain explicit finite experiment contracts. Cargo policy export is offline and
locked; the real installed-tool export succeeds for eleven encoder recipes.
The isolated C writer compile exposes a separate unresolved API drift, T1113;
its failed build log is retained and no measurement is claimed.

Permanent normal-suite regressions first reject the old compiler/policy caps
(30/60 seconds), partial-start cleanup and skipped encoder cleanup (3 seconds).
A subsequent repeated-cancellation regression fails against the initial cleanup
implementation, then passes when original retirement observation survives another
interrupt. All 21 focused/architecture tests pass, including exact bounded data
through a real child and existing command, reporting and example-compile controls.
The source audit estimates 53 functions, maximum 8; it is not native SonarQube
measurement (T1019). Benchmarks and nonessential tooling are coverage-exempt;
no application production function changed and no tests were removed.

The command sponsor admits a new process session before borrowing any pipe.
The caller owns bounded input, logs and output; pipe writes copy bytes, and the
pipeline releases its mmap view synchronously before closing that mapping. A
partially started pipeline always closes both original children. On failure or
cancellation, group stop precedes original join; repeated wait interruption does
not release the original obligation. Pipe/log scopes end after the join, including
post-join reporting failure. PID identity stays unreaped while stop is requested;
already reaped processes are never signalled. No native device is borrowed here.

These are trusted installed compiler/Cargo/codec tools, assumed to join their own
children normally and keep subprocesses in their original group. This is not a
proof for arbitrary daemonizing children. An externally force-killed Python parent
cannot run its finally blocks: the kernel owns remaining private pages, pipe
references and log inodes until those trusted finite tools exit or observe
EOF/SIGPIPE. Eventual cleanup assumes eventual scheduling/tool termination;
no parent retirement or unrestricted process-tree fence is claimed in that case.
No physical EVDI, tablet, battery, suspend or network activity was performed.

</details>
