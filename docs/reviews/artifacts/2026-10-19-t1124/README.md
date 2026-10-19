# T1124 original retirement observations

`blent_config::retirement::observations` is an independent bounded observation
contract. Reserve before native effects, then synchronously bind the resource's
exact original Completion before returning/yielding. Failed creation drops an
unbound reservation. Binding is linear and infallible; dropping a bound ticket
preserves its proof. A fresh Arc identity prevents stale tickets addressing a
reused slot. This table owns metadata, not native devices/cancellation authority.
Native owners still sponsor cleanup; observing completion cannot create it.

Safety invariants: at most N unresolved observations; every admitted reservation
has one table-owned identity; no slot reuse until its reservation is cancelled
before creation or its bound original proof completes. No retained native access
or borrowed view is introduced. Mutex poisoning preserves the original state.
Eventual slot reuse requires actual native completion and another admission;
no deadline or fairness assumption manufactures retirement.

Three permanent normal-suite tests include capacity zero/one, failed creation,
original identity reuse and poisoned-state preservation. The independent two-slot
model exhausts 32,768 five-step traces / 163,840 transitions over reserve, bind,
cancel and original owner release. It checks conservation and admission after
every transition. This finite model does not prove arbitrary native interleavings.
A deliberate fault treating bound proofs as complete fails these same tests;
exact source restoration passes. The full common library passes all 383 tests.

Fresh offline LLVM collection measures every production function individually:
7/7 functions at 100% executable lines, no exclusions. Source/compiled-object
attestation and raw counters remain at
`/backups/disk2/blent-work/2026-10-19-open-items/t1124/native`.
The pinned repository tree-sitter audit (0.25.2 / Rust grammar 0.24.2) measures
all 16 new production/test functions at <=6. This is not native SonarQube evidence
(T1019). No Windows/EVDI/ADB integration or physical-device validation is claimed;
T1125/T1126 and parent T1089 remain separate work.
