# T1117: original request retirement waits for filesystem evidence

Idle-request cleanup retains its original identities and retries until the filesystem confirms removal or replacement. Numeric cookie comparison also matches the C reader's accepted padded values. Bounded models and native fixtures verify no early release under the tested failures; eventual cleanup still depends on scheduling, cooperating locks and storage availability.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

Idle request cleanup now retains its original directory, FIFO and token identity
pins until coordinated namespace cleanup settles. An I/O error no longer permits
the destructor to release those identities. This changes cleanup of the existing
500 ms mode; longer-cadence adapter/configuration work remains T1115/T1116/T1111.

## Release contract

The existing native idle worker retains one original lease through its creator
thread join and final cleanup thread join. Lease destruction retries its cleanup
under the process mutex and original-directory native lock. It releases its
fields only after one of these observations:

- The request name is missing or contains a nonregular replacement that the C
  reader cannot consume as advice.
- Readable numeric source/token fields identify another request owner.
- Its own numeric cookie matches and unlink succeeds, or the kernel reports the
  request already missing during unlink.

Lock, inspection, read, parse and removal errors remain pending. The first error
is logged with original path/source/token context, and retries pause 100 ms.
No elapsed execution deadline ends this native obligation. A logical Stop only
revokes publication; existing original completion reporting remains unconfirmed
until actual cleanup and native joins finish.

This implementation conservatively waits for namespace settlement even after
advice's ordinary 1.5-second validity interval. It does not infer a release fence
from a clock, timeout, destructor entry or reference count. A missing/unavailable
clock therefore cannot weaken cleanup. Conditional eventual cleanup requires
eventual scheduling, cooperating lock owners releasing their locks, and storage
becoming readable/removable or being positively replaced. Unknown obligations
continue consuming their existing funded native work admission; no unbounded
new task, queue or native allocation is created.

The C writer owns its original FIFO descriptor before looking up advice and
keeps it throughout that lookup. A reader that already opened an old request
therefore still has its old source identity pinned. Unlink/displacement prevents
a new reader for a reused source from opening that old request through the
namespace. The lease's own pins close after the cleanup workflow returns;
actual native worker completion is reported after the cleanup thread joins.
These are advisory-file boundaries, not graphics-buffer or device quiescence.

## Numeric ownership and tests

The C parser accepts leading-zero decimal field encodings, while the former Rust
byte-prefix check disowned them. Rust now compares the three numeric cookie
fields against its original identity values. Matching noncanonical padding is
cleaned up; a different numeric cookie is preserved; unreadable/unparseable
regular storage stays pending. Parsing consumes three values from the existing
160-byte bounded read and retains no borrowed input or allocation.

Both permanent regressions fail before the behavior fixes: injected failed
removal returns after one attempt instead of the required three, and a padded
owned cookie remains in the namespace. Both pass afterwards. An independent
normal C fixture executes the unchanged production parser with real pipe identity
and padding widths 0, 1, 16 and 20; AddressSanitizer, UndefinedBehaviorSanitizer
and LeakSanitizer pass. Thus the accepted numeric encoding is independently
confirmed rather than inferred from the new Rust parser.

The bounded cleanup model checks every failure count from 0 through 256: 33,153
attempts in total, with exactly one successful settlement per trace and no early
return. Additional bounded cookie, overflow, invalid-value and unknown-storage
checks retain identity on failure. A native worker test publishes through the
real lease, introduces unreadable storage, observes the original cleanup receipt
still pending, repairs storage, then waits for actual original completion and
checks the request removed. Its 150 ms pending observation is supporting native
evidence; the deterministic injected-failure model is independent of scheduling.
All test callbacks are synchronous scoped borrows; fixtures settle resources
before assertions and teardown.

All 30 normal idle tests pass. A deliberate single-attempt retirement fault is
rejected by the permanent removal regression. After restoration, changed source
bytes match the fresh offline LLVM coverage manifest. All 32 production functions
in the idle/directory scope measure 90.9–100% executable-line coverage individually.
Source estimates cover 76 Rust functions (maximum 7), the Python fixture method
(1) and C fixture main (2). These are repository/manual estimates, not native
SonarQube evidence (T1019). Five architecture tests pass. Raw evidence is in
[the artifacts](artifacts/2026-10-19-t1117/README.md).

Final-request cleanup allocates no persistent record or new allocator. Each
iteration owns at most its bounded read/error/context and drops it before the
next attempt; original descriptors stay owned throughout. Rejection and partial
creation unwind their existing owned fields. There are no retaining cycles.
Cancellation hands ownership to the existing native reaper instead of detaching
cleanup. Abrupt whole-process loss is not a successful cleanup receipt; T1115
must reject/settle stale advice before exposing a replacement source/reader.

Subsequent [T1118](2026-10-19-idle-staging.md) resolves temporary publication
cleanup with a borrowed staging owner and guarded recovery. This T1117 evidence
covers the separate final-request fence. No physical
display, tablet, suspend, application installation, network or battery action ran.

</details>
