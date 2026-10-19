# T1118: staging ownership stays inside the original directory guard

Idle publication now owns its temporary file through a `Staging` value borrowing
the original native directory guard. Its narrow mutable Write view borrows that
owner; persistence consumes it. An owned temporary pathname cannot escape through
an I/O error and later act on a reused directory descriptor.

Allocation creates one exclusive regular temporary file under the guard, with
the reserved `.blent-idle-stage-` prefix. The production caller writes its existing
bounded five-field ASCII record. Successful persistence atomically transfers the
named record to the lease's final-request namespace; the returned File closes
under the same guard. The staging owner then holds no cleanup obligation.
Failure restores its temporary file to the owner, whose destructor must settle
cleanup before the plain original I/O error returns. Early return, partial write
and invalid target use the same cleanup path.

Cleanup retains the original File and pathname together. It compares the opened
file's native device/inode to the current non-followed pathname. Missing or
different identity settles the original pathname obligation without deleting a
replacement. Otherwise unlink must succeed or report the name already missing.
Other errors retry through the existing retained workflow without an elapsed
kill cap. The file remains open through every attempt, preventing premature
inode reuse. Implicit TempPath deletion is disarmed when this explicit owner
takes over: it must not perform unobserved I/O or unlink a replacement a second
time. Path memory and File close after settlement while the directory still lives.

This is one authoritative owner, with no detached callback, new pool or duplicated
mutable file state. The original admitted creator/worker/cleanup sponsor retains
unknown obligations and descriptors until storage progresses. Conditional eventual
cleanup requires scheduling, cooperating lock owners and readable/removable
storage; cancellation and reference counts do not replace native completion.

## Recovery after owner loss

Every current staging allocation and retirement holds its native directory lock.
`Lease::open` acquires a **fresh open-description lock before local staging
allocation**. This excludes other live current staging owners. Pre-admission
collection uses that ordering, not a PID or age guess, to recover abandoned
reserved entries. Its private caller contract forbids collecting while that
same guard already owns a local stage.

Collection streams entries and repeats until a full pass reports no reserved
stage. Candidates must be regular, owned by the current user and not writable
by others. An opened descriptor pins the original identity before unlink.
Other names are preserved, symlinks are not followed, and untrusted reserved
objects reject admission. Nonparticipating same-user mutation is outside the
private coordinated namespace contract; this is not general compare-and-delete.

After abrupt process loss, the kernel closes descriptors and releases the native
lock after its last owner; remaining tagged links are explicit runtime-namespace
debt. A later guarded admission settles them before allocating further stages.
Validation/I/O failure rejects new staging allocation and preserves that debt.
Conditional eventual recovery requires later admission/storage availability or
runtime-namespace teardown. No receipt is invented for a dead process, and generic
historical `.tmp` files are not blindly claimed or deleted.

## Permanent evidence

Two regressions fail first against destructor-only cleanup: injected unlink
failure is ignored (zero observed attempts instead of three), and a foreign
replacement disappears. Both pass with explicit ownership. During failures, the
test checks the original descriptor's actual native device/inode stays present.
A deliberately omitted identity check fails the replacement regression; source
is restored before final checks.

The owner-loss regression first leaves five namespace entries instead of the
expected unrelated entry alone, then passes with collection. A real Rust child
also creates a tagged stage and calls `process::exit`, bypassing stack destructors.
Its original native completion is observed before a fresh guard recovers the
file, checking actual kernel descriptor/lock release.

Tests also cover persistence, invalid targets, already removed entries, native
missing-entry results, 257 abandoned files mixed with 257 unrelated entries, and
directory/symlink/insecure reserved objects that must remain untouched. The mixed
test checks streamed traversal/final-pass verification, not machine/tablet
performance. Existing failed-persist/non-escaping-error and original creator,
worker and cancellation regressions remain intact.

All 39 normal idle tests pass without failures or skips. Fresh offline LLVM
counters measure all 45 production functions in idle/directory/staging
individually at 90.9–100% executable-line coverage. Source estimates cover 98 Rust
functions, maximum 7 (tree-sitter-rust 0.24.2/tree-sitter 0.25.2); native SonarQube
remains T1019. Five architecture tests pass; the ordinary executable checks
without warnings. Restored source hashes match native coverage inputs.
[Raw evidence](artifacts/2026-10-19-t1118/README.md) retains failures, counters,
owner-loss validation and the faulty control.

T1115/T1116 still own cadence integration, replacement-source stale-advice
admission and shared capture; this does not enable longer cadence. No installed
application, physical display/tablet, suspend, network or battery action occurred.
