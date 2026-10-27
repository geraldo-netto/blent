# T1226: preserve audio retirement across private IPC Stop

Audio cleanup messages remain deliverable after private IPC Stop, with exact generation and queue bounds preserved. Hosted tests passed; signed-APK construction alone did not complete physical audio acceptance.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The shared actor and audio contracts permit exact original consumer retirement
while stopping. Two native IPC boundaries still implemented the older camera-only
rule: AndroidConnectionWorkerRun.command sent audio proof through the ordinary
queue, which closes at Stop; ConnectionWorkerService.control/controls rejected or
dropped the same message. A native audio owner could therefore remain waiting
for its original completion or lose its service before protocol settlement.

Permanent API27/34 private client and Binder service regressions were added before
the behavior change. All four selected executions failed; `red.log.gz` retains the
client rejection and service IllegalStateException. Both independent directions
are exercised. The shared isConsumerRetirement classification now serves the
client, service and existing serialized actor. The actor's duplicate private
classification is removed; there is no retained alias or compatibility wrapper.

The final normal cohort passes223 tests with zero failures/errors/skips. Original
camera, audio ownership/codec/consent and joined protocol cases remain. New audio
controls reject before Start, after Ended/death and with a foreign Binder
generation; ordinary consent stays rejected after Stop. Both directions exercise
the32-entry cleanup queue, rejection on exhaustion, failed delivery and a held
native IPC operation whose ownership survives death until the original call
actually returns. The same method-line counters show all six changed/new
production functions at100%, with every mapped native variant passing. Exact
source hashes, authored inventory,58 class identities, raw JaCoCo0.8.15 XML/exec
and test reports are retained. Kotlin PSI1.9.20 estimates488 production/test
functions, maximum8; these are source estimates, not native Sonar evidence.
Native SonarQube remains unavailable; no tool was downloaded.

The signed production APK is built offline and verified against the existing
release certificate. `apk.json` identifies its exact bytes and current deployment
state. Native device deployment/pairing and physical T860/T859 acceptance remain
separate from Robolectric API27/34 simulations and the private protocol fixtures.
The device is currently in debugging-off accessory mode; updating its APK needs
the owner's temporary USB-debugging switch before debugging-off acceptance resumes.

The new predicate is stateless and borrows one validated command for the call;
it allocates and retains nothing and grants no native authority. Existing exact
lease/generation checks,32-slot queues, blocked IPC jobs, Binder death/unlink,
original endpoint/consumer claims and actual retirement barriers remain the
owners. Stop cuts off ordinary work while cleanup stays sponsored by the original
worker. Rejected, stale, ended or dead owners cannot create/bind replacements.
Death alone does not release a blocked original IPC call. Tests retain its call
until explicit release and final retire/unbind; conditional cleanup assumes
native return and scheduler progress. Hosted counters and bounded lifecycle
fixtures do not prove unrestricted native safety or physical acoustic behavior.

</details>
