# T1122 native fixture ownership

The permanent normal-suite Rust regression constructs two Harness owners with
exactly the same native case label, requires distinct directories, drops the
second and executes the first's original binary. Before the fix both paths were
identical (red.log). After the fix, the complete normal EVDI helper suite passes:
61 tests, four test threads, zero failures/ignored. Existing ASan, UBSan, LSan
and TSan selections remain enabled. Native execution ran outside the sandbox
because sandbox ptrace prevents LSan operation. No hardware or network action.

Each Harness owns a private unique TempDir. Compiler output() returns before
build success/failure is inspected; native output() returns before the Harness
can leave scope. TempDir handles normal return and panic cleanup; dropping one
owner cannot unlink a concurrently retained owner's binary. A killed test
process can leave its private directory for OS/temp-namespace cleanup; this is
not a promise of arbitrary process-tree or device retirement. Compiler/native
fixtures are trusted not to daemonize. Production native ownership is unchanged.

Only test code changes; no production coverage exemption is added. Complete
Rust source audit: 63 functions, maximum estimated complexity 4, cached
 tree-sitter-rust 0.24.2/tree-sitter 0.25.2 and repository rules. This is not native
SonarQube measurement (T1019). This checkpoint precedes T1119 native integration.
