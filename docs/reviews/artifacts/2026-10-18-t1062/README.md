# T1062: bounded original power request admission

Independent producer/receiver contracts carry opaque original owners, immutable
desired policy/withdrawal views and original notice receipts. The implementation
retains one latest request, one dirty bit and one admitted notice across closure
and replacement. Changes coalesce without creating another queued notice. The
original receipt cannot apply another owner's policy; duplicates/foreign notices
cannot acknowledge a successor. Closure is terminal for its owner, while exact
current withdrawal remains a relevant stop view. Snapshot relevance is checked
by identity, independently of wire/process counters. Native integration is T1060;
Activity window/current-source integration T1061, joined acceptance T1058/T1011
and physical host/tablet evidence T914 remain unresolved.

The [contract](../../../display-wake-policy.md#original-request-admission) records
initialization/transition invariants, ownership, bounded retained values, cutoff,
receipt, conditional eventual cleanup and native refinement gaps. Unknown delivery
retains original admission; elapsed time never authorizes replacement. Logical
receipt is not native lock/decoder permission, release or reuse proof. The backend
must separately serialize current-authority validation/effects and account for
actual original handoff/native settlement. Pure value processing introduces no
native handle, OS dependency, callback, coroutine or retained Context.

Permanent focused tests cover terminal closure, current/stale snapshots, foreign
owners, duplicate receipts and 1,000 coalesced changes. An independent model exhausts
all 4,096 four-step histories over eight actions and two producer slots, checking
original authority, one queued notice and retained view validity after every step.
The foreign-owner faulty control fails the permanent original-notice regression
before its fence is applied. That historical control precedes extraction of the
independent contract types; its raw source/log are retained. This is a deliberate
control for a new contract, not a claim of reproduced native service failure.

Fresh offline ordinary suite: **2,046 tests, zero failures/errors/skips**. All 14
authored production methods independently measure **100%** native JaCoCo 0.8.15
line coverage. The pinned Kotlin PSI 1.9.20 inventory retains existing inline and
compiler-generated attribution, with no new exclusions. Source/vector hashes,
native class hashes, raw execution/XML counters and individual reports are saved.
Cached source audit covers 36 production/test functions, maximum 9; this is an
estimate, not SonarQube certification (T1019). Twelve architecture/contract/
portability checks pass. No network transfer, installation, push or live host/tablet
interruption occurred.
