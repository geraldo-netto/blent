# T1147 — cancellable output admission

Camera device open/label validation now runs in an admitted owned blocking
constructor. Stop and abandoned reply ownership are checked before each endpoint
and before handing off the pair. The coordinator can process Stop while an
original open/read remains blocked; it skips later endpoint/connection admission.
Pre-stopped or closed control starts nothing. Native operations already accepted
remain owned until actual return and join. Existing ADB creation/removal behavior
is unchanged; cancellation of an already accepted ADB transaction remains T1090.

Report reserves two device observations before effects: original constructor
completion and separately funded original-file cleanup. The constructor also
reserves the file-close work credit before opening anything. Partial files close
on the native constructor thread. A complete pair has one Devices owner; its
bounded two-file view may be cloned only into separately observed original workers.
Drop transfers those original references and their admitted credit to existing
native Cleanup work, including queued-result rejection, late delivery and owner
loss. It does not close files on the async coordinator. Producer observations
separately cover any worker-held views. The complete Report, rather than a single
resource group's observation, gates replacement. No new callback or unbounded
queue exists; reply capacity is one pair and each path is copied once.

Conservation: each admitted native constructor/close obligation remains either
owned, retained by its original cleanup sponsor, or actually settled. Replacement
must wait for actual retirement of every original accessor. Actual join
and dependent cleanup settle work credits; cancellation, logical completion and
reference counts alone do not. Eventual cleanup assumes native return and fair
retained observation. A permanently blocked open/read/close remains funded.

The permanent sealed private-mount/PTY/sysfs-FIFO regression fails before the fix
and passes unchanged afterward. An independent writer requests Stop while the
original native label read is blocked, waits for coordinator acknowledgement and
then releases the FIFO on both acknowledgement and protocol failure. Actual native
cleanup precedes assertions and fresh original-lock probes. Inotify confirms the
later endpoint's label was never opened, and a private fake command confirms no
connection was created. No live ADB, terminal or camera is used.

Additional normal-suite tests cover pre-stopped/closed control, abandoned reply,
partial pair failure, original observation and work-credit exhaustion at both
admission stages, queued pair loss and actual original file-lock release. The
270-case invalid-path corpus includes all 256 single-byte components (NUL and
invalid UTF-8 included), symlinks, directories, missing/regular/non-video nodes,
component boundaries 255/256 and lengths through 4097 plus the fixed test prefix.
The retained T1088 failed-cleanup oracle now requests Stop after its private fake
mapping is admitted, using a kernel gate; pre-stopped startup correctly creates
no mapping. Its original failure assertion and cleanup remain intact.

Offline current-source validation: 98 default camera tests and 981 accessory
host-library tests pass; 3 existing benchmark ignores remain. The same attested
instrumented ELF admits both real v4l2loopback 0.15.3 devices in a networkless
Linux 7.0.0-34 guest, runs the production writer path, matches original black
frames and independent C metadata, drops the pair, waits for all native proofs
and reacquires both original locks. Module unload and actual VM exit pass.
Combined native coverage passes all 107 camera/control production functions
individually at least 80%, none unmeasured. The eight new admission functions
range from 94.4% to 100%. This is isolated driver/pipeline evidence, not physical
tablet/browser acceptance or a proof of unrestricted kernel safety.

Offline host+GUI builds, scoped Clippy and changed-source rustfmt pass. Pinned
parser source audit: 67 functions, maximum 5; native SonarQube remains unavailable
under T1019. No network transfer, push, install or host service/module/device
change occurred. T1090 and parent T1089 remain unresolved. Raw source manifests,
attestations and counters remain under
`/backups/disk2/blent-work/2026-10-19-open-items/t1147`; `export.json.gz` binds
accepted sources, native objects, guest and profile hashes.
