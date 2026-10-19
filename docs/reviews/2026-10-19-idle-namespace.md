# T1114: original idle namespace coordination

Idle request publication and cleanup now use a pinned original directory and a
native directory `flock`, alongside the existing process mutex. The permanent
regressions first demonstrate old cleanup deleting a foreign publication between
its ownership snapshot and unlink, and abandoning its original request after
the parent pathname is renamed/replaced. Both pass with the fix.

The directory is opened before looking up the FIFO. FIFO lookup, token admission,
temporary publication, reads and unlink use its `/proc/self/fd` view. A changed
parent pathname therefore cannot retarget an effect. Only its single final file
name is admitted; empty/root/dot-parent names reject. The native adapter checks
directory ownership and excludes group/world writable directories, using
`O_DIRECTORY`, `O_NOFOLLOW` and `O_CLOEXEC`. Existing runtime directories already
have private permissions. Idle test fixtures now set their private modes
explicitly rather than relying on installed tempfile/umask defaults; assertions
and existing lifetime tests remain intact.

All current application publishers and cleanup paths take locks in the same
order: process mutex, then original-directory native lock. This coordinates
independent open file descriptions and processes without creating a persistent
lock file. The private namespace is an application coordination boundary;
advisory locks do not exclude arbitrary nonparticipating same-user mutations.
No claim of kernel compare-and-delete against such mutation is made. File
ownership checks and original-directory pinning still apply on every effect.

The lease owns its original directory, FIFO identity pin, token identity pin and
stop view. A native guard borrows the directory File only through a synchronous
effect. A publisher blocked acquiring the native lock rechecks original Stop
after acquisition; already admitted native effects remain owned through their
actual completion and existing original worker retirement. Removing either the
cleanup lock or post-wait Stop check deliberately fails its permanent regression.

Atomic publication writes a bounded ASCII request (five existing numeric fields)
to an owned temporary file. A failed `persist` contains an owned temporary path:
that path is destroyed under the original directory borrow before returning its
plain I/O error. It cannot escape and later dereference a reused directory FD.
Success closes the returned publication File under the same guard. Partial
startup and validation rejection unwind owned FDs. Failed persistence settles
its temporary file while the lease keeps its directory/source/token pins; no
new queue, callback registry or persistent lock inode exists.
Test callbacks are synchronous borrowed barriers, never detached work.

Twenty-five normal idle tests pass, including six new cases for the exact
snapshot/publication/unlink interleaving, directory replacement, Stop during
native lock wait, persistence failure, bounded path/descriptor/permission inputs,
and real Python child-process lock exclusion/unlock. Structured native test
threads join before fixture cleanup, including unwinding. Real process probes
reuse `OwnedChild`/`Scope::group` and retain original native observations through
actual reaper settlement before assertions or runtime teardown. The five-second
interleaving handshake and two-second child-probe protocol bounds are fixture
acceptance contracts; native cleanup and compilation have no elapsed kill cap.

Fresh offline LLVM counters measure all 27 production functions in the idle
adapter and directory module individually at 90.9–100% executable-line coverage.
The source estimate covers 66 scoped functions, maximum 7 (tree-sitter-rust
0.24.2/tree-sitter 0.25.2); it is not native SonarQube evidence, still T1019.
Cargo LLVM coverage is 0.8.7. Five architecture tests pass. Both faulty controls
are restored byte-for-byte against the native coverage manifest before final
normal verification. Raw evidence is in
[the artifacts](artifacts/2026-10-19-t1114/README.md).

This resolves namespace clobbering and retargeted cleanup. **T1117 remains open**:
failed removal can leave short-lived advice after the lease's source/token pins
close. That separate expiry fence must be fixed before longer advice is enabled
by T1115/T1116. A completed destructor or expired logical owner does not prove
that boundary. Existing original idle creator/worker/native completion regression
coverage remains; this change does not claim unrestricted leak freedom, native
device support, or safety against uncoordinated external namespace writers.
No installed application, display, suspend, tablet, network or battery action ran.
