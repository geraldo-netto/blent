# T1154 — admit persisted trust lists before copying

The persisted trust constructor copied a caller List before checking its 1024-peer
limit. The permanent normal-suite regression supplies a 1025-entry probe that
records element access: rejection must occur before any element is read or copied.
It fails with the original implementation and passes unchanged after the fix;
its whole-file hash is sealed. The bounded copy helper validates maximum and
count before allocating a list, copies at most that admitted count by index and
retains no input list/iterator/callback. A failed read publishes no partial result.

Copies contain immutable peer values and have JVM-managed lifetime. No native
resource, asynchronous handoff or independent mutable trust owner is introduced.
Existing canonical order, revision, identity, stored wire bytes and publication
failure rules remain unchanged. The active T1151 page draft also uses the helper
with its 48-entry bound; this does not qualify that unfinished adapter.

Permanent bounded properties cover every valid count 0..1024, rejected maximum
values including signed Int extremes, over-limit counts at 0/1/48/1024 limits and
an injected mid-copy failure. Existing trust/codec/admission tests remain intact.
24 tests pass without failures, errors or skips. Current JaCoCo 0.8.15 counters
pass all 11 combined saved-view/copy/persisted-codec production functions and
mapped native variants individually at least 80%. The pinned Kotlin 1.9.20 PSI
audit covers 73 production/test entries, maximum 5; native SonarQube remains
unavailable under T1019. The shared source snapshot contains pending T1151 work;
The persisted-codec source matches this commit; saved-view/copy sources are
unchanged from T1153. No pending private-process adapter is qualified here.
Raw shared XML/exec/source manifest are retained in the T1153 artifact; compiled
class hashes, per-method results and the original failing oracle remain here.
No device, installation, network transfer or push occurred. Local raw evidence:
`/backups/disk2/blent-work/2026-10-19-open-items/t1154`.
