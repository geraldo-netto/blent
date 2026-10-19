# T1156 — preserve the original kernel trust lock

A rejected second JVM open closed another descriptor for the same `trust.lock`
inode. POSIX process locks disappeared even though the original Java FileLock
still appeared valid. The permanent regression uses an independent Python
`fcntl.lockf` process and the original device/inode identity. Before the fix,
the second probe acquired the lock; after the fix it cannot. Only actual original
store close permits the final probe to acquire it. The unchanged Kotlin/Python
oracle hashes, original failure and current native counters are retained here.

Admission now reserves the original directory's NIO fileKey before opening a
channel. Aliased paths share admission; unrelated directories remain independent.
The process registry admits at most 64 original directories; its internal
constructor accepts 1..1024 for boundary verification. Missing stable fileKey,
duplicate ownership and capacity exhaustion fail before opening child files.
The private, stable application-directory requirement remains; this does not
provide isolation against other code with the same UID or qualify every Android
filesystem provider. The existing OS lock still excludes independent processes.

Each registry entry has one immutable native identity and a fresh object token.
Only its original lease releases it, after both original channel closes return
successfully. Close failure retains admission until the containing private
process exits; cancellation, timeout, Java lock validity and logical completion
are not release evidence. Repeated old close cannot erase a replacement entry.
An injected directory opener owns any failed acquisition and must not throw
while abandoning an unreturned channel. Production uses the stock NIO opener.
There are no background tasks or finalizers in the registry. Conditional eventual
cleanup requires actual original native close success or original process exit;
neither fairness nor native-close success is established by hosted counters.

Ten tests pass with no failures, errors or skips: the native lock regression,
alias/independent-directory and acquisition/unknown-close faults, 1/2/64/1024-slot
exhaustion, invalid capacities, forged/stale release and 32 concurrent requests,
plus the existing native storage/publication-failure suite. Probe children are
actually reaped and original stores closed before assertions and directory reuse.
This is bounded transition/fault evidence, not an exhaustive interleaving proof.

JaCoCo 0.8.15 passes all 19 scoped production functions/native variants individually
at least 80%. The cached Kotlin 1.9.20 PSI audit reports 66 production/test entries,
maximum 4; it is not native SonarQube evidence (T1019). The source snapshot includes
pending T1151 work, which is not qualified by this scope. No app/device action,
installation, network transfer or push occurred. Raw local evidence:
`/backups/disk2/blent-work/2026-10-20-open-items/t1156`.
