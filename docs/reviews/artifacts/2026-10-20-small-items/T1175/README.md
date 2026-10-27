# T1175: terminal attachment epochs

Attachment epochs stop permanently at exhaustion instead of reviving stale leases. Boundary and wider-integer tests passed; terminal authority does not certify native resource retirement.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Legacy and registered attachments now share one checked generation transition
inside the original state mutex. Overflow permanently closes authority, clears
credential/geometry availability and notifies original retirement watchers without
wrapping. Normal retirement also pins the maximum; repeated retire/begin cannot
restore it. Current public interfaces and native cleanup ownership stay unchanged.

Three permanent normal-library regressions fail before the repair: stale zero
leases revive, terminal retirement wraps to0, and final handoff leaves authority
open. They pass after repair. The complete attachment scope passes17 tests,
including registered-route identity/credential/retirement tests, exact terminal
watch notification and poison recovery. A bounded independent u128 oracle checks
130 initial values (0–64 and MAX−64–MAX), each through70 handoffs:9,100 transitions.
The oracle is numerically independent of the production checked-add branch.

LLVM counters in a fresh exclusively owned offline target report100% executable
line coverage individually for all six changed production methods. Raw LCOV,
commands, source fingerprints and original red/green output are retained here.
Authored-source decision/closure estimates are <=6 for production/tests; actual
native SonarQube evidence and unavailable tree_sitter grammar remain T1019.

The existing mutex/state owner still owns identity, credentials and generation;
leases retain original immutable snapshots and Arc ownership until actual drop.
Terminal numeric invalidation prevents new work but does not certify native
child/encoder/device retirement. Existing creators, jobs and cleanup sponsors
remain responsible for those original obligations. No descriptor, allocation,
worker, queue, callback or buffer reuse path is added. Rejection leaves no fresh
lease; existing leases remain owned until their original users release them.
Poisoned actions can only revoke. These bounded hosted tests do not prove every
native interleaving or unrestricted leak freedom. No physical device or service
was changed, and no Windows/EVDI backend was edited.

Review also found separate decoder_epoch wrapping in unchanged
EncoderSettings::clear_decoders. T1187 records that distinct settings/selection
owner; it is not silently included in this attachment repair or declared fixed.

</details>
