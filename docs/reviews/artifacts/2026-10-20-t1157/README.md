# T1157 — preserve original identity-file ownership

Identity loading prevents duplicate and hardlinked lock opens from releasing another call's POSIX lock. Native lock and partial-failure tests passed under a private stable-namespace assumption; physical Android providers remain separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

A rejected second same-JVM identity load closed another descriptor for the
original inode and released its POSIX process lock while the first provider call
was still blocked. The independent-process regression records the original
device/inode, verifies held/rejected/held, then requires actual original thread
return and close before the final unlocked probe. Before the fix the second
probe acquired the lock. The unchanged Kotlin/Python oracle is sealed.

The initial directory guard still allowed a hardlinked lock in another private
directory to open the original inode. Its separate native regression failed
before child-file validation was added. That unchanged oracle is also sealed;
the intermediate guard source and failure remain here. Both regressions settle
the original loader/lock and actually reap probe children before asserting.

The identity adapter now reserves the stable original directory fileKey before
opening its lock file. It reuses the bounded, fresh-token directory registry with
an independent identity-loading registry, default 64 slots. Alias paths reject
before descriptor creation; unrelated directories remain available. Existing child
files must be regular, owned by the directory owner and have one hard link.
Symlinks, hardlinks and other unsafe state reject before opening, with validation
repeated after acquisition. Missing stable fileKey fails visibly. The namespace
must remain application-private and stable across checks/open; this is not a
sandbox against other code with the same UID. All in-process opens of the
identity lock must use this adapter. The injected opener returns the original
file for that exact path and owns its failed acquisitions; production uses stock
RandomAccessFile. The empty lock file's mode is preserved inside the private
directory; no key material is stored in it.

One adapter owns the file, FileLock and exact registry lease throughout the
synchronous provider load. Successful original file/channel close is the native
fence, including after a failed explicit lock release. Native close failure
retains the slot; a repeated logical close cannot clear it. Failed acquisition
closes the original returned file before returning its lease; cleanup failure
preserves the original error, adds the cleanup error and retains admission.
Unknown obligations stay funded until original private-process exit. Conditional
eventual cleanup requires actual native return/close or that original process
exit; no timer, Java validity flag or GC event supplies proof. Persistent provider
aliases and returned TLS key-manager lifetime remain with the existing identity/
TLS owners; closing this file lease never deletes or rotates a key.

Permanent tests cover original/alias lock loss, independent foreign-process
contention and subsequent admission, invalid directory/child/owner metadata,
acquisition/close/lock-release faults, partial-failure error preservation, bounded
registry exhaustion and stale/forged/concurrent leases. The foreign-owner control
uses a typed unrelated principal; no privileged UID change is performed. Python
fixture tests bound argument/path/64-bit identity values and verify exact marker,
single-byte release/rejection and actual descriptor-close calls. Tests use fresh
private namespaces, synchronous original files and actual child exit/reap. This
is bounded state/fault/native evidence, not an exhaustive interleaving proof.

All 43 selected JVM tests pass without failures, errors or skips; exact totals and
raw native counters accompany this record. JaCoCo 0.8.15 passes all 21 scoped
production functions, including every mapped native variant, individually at least
80%. Seven Python/architecture checks pass. Offline lint and APK assembly pass;
the existing nine lint warnings remain T1159–T1166. Cached Kotlin 1.9.20 PSI covers
117 production/test entries, maximum 5; a conservative Python source estimate
covers six functions, maximum 8. Neither is native SonarQube evidence (T1019).
Linux filesystem/JVM and hosted Android results do not qualify physical Android
providers or unrestricted native safety (T863). No live app/device, installation,
network transfer or push occurred. Raw local evidence:
`/backups/disk2/blent-work/2026-10-20-open-items/t1157`.

</details>
