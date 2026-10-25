# Temporary ADB media mappings

Blent creates a temporary ADB connection for each camera, microphone or speaker
session that selects ADB. Owner decision 2026-10-25: Blent manages the mappings
it creates until their actual cleanup finishes. Other programs may install apps,
read logs and use unrelated mappings. During that lifetime, they must not replace
or remove Blent's temporary endpoints, or run `adb reverse --remove-all`.
Direct USB media uses its own adapter and does not create these mappings.

Each mapping reserves capacity before native creation and retains one original
device serial, remote endpoint and local destination. A host loopback listener
reserves the chosen numeric endpoint until cleanup. The lease also owns a native
duplicate of the actual local listener, preventing destination-port reuse after
caller cancellation or runtime teardown. The Android endpoint is
checked for conflicts and created with `--no-rebind`. An existing route is never
adopted or overwritten. This replaces dynamic `tcp:0` allocation whose endpoint
identity could be lost when its creator was cancelled.

`BLENT_ADB_MAPPING_LIMIT` defaults to 16 and accepts 1..4096. Set it in the host's
launch environment; restart the host after changing it. Raising the limit admits
more simultaneous or unfinished mappings and retains more threads, sockets and
memory when cleanup stalls. Exhaustion rejects creation before mapping effects.
The invitation queue holds at most one waiting request and one executing request;
additional concurrent requests reject. Encoded invitations are bounded to 4 KiB.

Creation, invitation and cleanup run on one retained worker per mapping. It has
its own runtime, so cancelling the caller or destroying that caller's runtime
does not destroy cleanup ownership. Camera and audio controllers record the
original completion before native effects and wait for actual retirement before
replacement. Close cuts off new invitations atomically with admission; queued
requests are discarded when stopping. Already executing work stays owned.

Commands keep their existing five-second protocol deadline. The worker observes
the exact child and output-collector completion tickets before another command
or snapshot is interpreted. A caller's five-second close wait may return an error;
the worker, mapping admission and endpoint reservation remain retained. A deadline,
cancelled reply or logical session result is not cleanup proof.

The platform-independent policy in `common/src/adb_reverse/temporary.rs` defines
four creation states and three snapshot classifications. Listings are bounded
to 64 KiB and malformed/duplicate endpoints reject before mutation.

| Original state | Missing route | Matching route | Foreign destination |
| --- | --- | --- | --- |
| No creator started | Complete | Preserve | Preserve |
| Creation outcome unknown | Retain and observe | Remove the original single effect | Retain and preserve |
| Creation acknowledged | Complete | Remove | Preserve; original route is gone |
| Removal outcome unknown | Complete | Retain; do not issue another removal | Retain and preserve |

An empty snapshot cannot settle an unacknowledged creator that might publish its
single delegated effect later. An uncertain removal is not reissued while its
first effect remains unknown. Under the exclusive endpoint contract, observing
that original effect supplies the corresponding state transition. Native command
completion is required separately. Read-only observation retries every 100 ms;
errors preserve ownership. Successful settlement closes both owned listener descriptors,
then joins the worker and releases admission. A worker panic leaves its original
reservation/admission quarantined instead of asserting cleanup.

Conservation: every started mapping worker owns one reserved admission; each
command owns at most one child and one collector observation; no admission is
reused before native settlement, worker destruction and join. Metadata views
borrow the original lease and cannot admit native work after cutoff. Queued
command bodies are owned values. There are no borrowed native pointers or
callbacks escaping a creator. The reaper owns the worker and original control
record; that record owns no reference back to the reaper, avoiding an ownership
cycle. Closed lease values may retain metadata, but no owned listener descriptor.

Eventual cleanup is conditional on the host process remaining alive, native
command/collector joins eventually completing, valid listings becoming available,
and unknown delegated effects eventually becoming observable. A permanently lost
creation/removal outcome can retain capacity indefinitely; arbitrary time passing
cannot make it safe to reuse. This is process-lifetime ownership, not durable
recovery after an abrupt whole-process or operating-system failure. Native ADB
must execute each submitted mutation at most once and return successful replies
only after applying that effect. External replacement during the admitted
lifetime violates the accepted contract; snapshot checks alone cannot prevent
that race.

Permanent T1090 tests execute the actual Rust adapter against private native
Python CLI/server fixtures, with separate ownership and join of the fake server.
They cover partial creation, late delegated effects, failed/abandoned close,
runtime teardown, foreign/missing/malformed routes, capacity, queue pressure and
endpoint release. The finite policy model checks all 12 state/route combinations;
invalid listing/port bounds retain independent oracles. These checks establish
the tested host behavior; they do not claim physical tablet acceptance or prove
unrestricted native ADB behavior. Existing platform support labels remain.
