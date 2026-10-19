# T1120 original native reader admission

Before the fix, the permanent C writer fixture accepts preexisting advice whose
source identity deliberately equals its new source descriptor. At 1200 ms it
sends only the initial picture instead of compatibility's second picture. The
independent scripted condition wait makes this deterministic; it does not rely
on persuading the kernel to recycle a particular inode. red-owned-root.log
retains the same failure with parent-sponsored private fixture storage.

After the fix, the original advice object is pinned before the writer's first
source access and rejected throughout that native reader's lifetime. Atomic
current publication with a different inode is accepted. No request is deleted.
Native fixtures retain these cases, descriptor identity across unlink/new
publication, repeated begin, original close, real numeric descriptor reuse,
repeated close preserving that new owner, retired-reader rejection, zero-init,
missing/null/unsafe/oversized/symlink/directory/ENOTDIR inputs, actual per-child
RLIMIT_NOFILE exhaustion and visible actual-writer compatibility fallback.
Existing 500/1000 ms cadence fixtures now admit their reader before atomically
publishing valid current advice; they retain their interval/deadline assertions.

Final normal native suites: 65 helper + 1 separate-module + 1 frame-retirement,
four test threads, zero failures/ignored. ASan, UBSan, LSan and TSan selections
remain unchanged; native runs outside the ptrace sandbox. Every production
definition in both changed C units is independently inventoried by Clang 18.1.3
and measured by GCC/gcov 13.3.0: 22 functions, 21 at 100%, writer_run at 95%.
Raw notes/counters/compile commands/reports and all native source hashes remain.
An unused instrumented compilation's missing-data warning is retained, never
credited as execution. Existing native writer ownership/compile (2), padded
cookie (1) and architecture (5) checks pass.

Deliberate faults accepting old advice, disabling all current advice, and
retaining a released descriptor identity each fail this permanent regression.
Finally restores exact original bytes; every native input hash is checked before
the restored regression passes. Source estimates: 112 project functions,
maximum 9, plus 4 collector/audit/fault functions, maximum 6. These are not native
SonarQube measurements; T1019 remains missing. No new exclusion or suppression.

The native resource/ownership proof and its assumptions are in the linked
review. This does not qualify shared capture, enable one-second settings, change
unrelated media or establish hardware/render/battery acceptance. No network,
live EVDI or tablet operation occurred.
