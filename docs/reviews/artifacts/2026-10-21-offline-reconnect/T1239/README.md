# T1239 offline reconnect, Stop and cleanup audit

The offline reconnect audit verified independent media Stop/receipt ordering and found the separately fixed stale-control cutoff race. Bounded models and native fixtures do not attribute the earlier uninstrumented physical episode.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Scope: original Android consent/access, binding attempts, consumer sessions and
completion relays; Linux media invitation/registration and retained native IPC
cleanup. `sources.json` pins the examined sources. The audit found T1241's
stale-control failure cutoff; its independent fix and24 permanent red/green cases
are committed as8af17124. No second confirmed production defect was found in this
scope. This does not prove absence of defects outside the examined contracts.

## Permanent bounded tests

`ConnectionReconnectStopTest` exercises all36 combinations of three-resource
Stop order and later original-receipt order, plus6 reconnect receipt orders.
It uses camera/microphone/speaker together, checks unaffected sibling claims,
exact terminal commands, duplicate proofs, and replacement peers with equal
lease values. Old local Start/Stop/authority callbacks cannot operate on the
replacement, and new offers require fresh consent. Original worker retirement
remains gated until explicitly supplied by the fixture.

`ConnectionStopInterleavingsTest` enumerates all720 permutations of six events:
three Stops (each followed by its original published route closure) and three
original consumer-completion receipts. Receipts may precede Stop. An independent
model requires both closure and completion before exactly one original terminal
command; unrelated owners remain current. Foreign-lease closure and duplicate
completion are checked in every trace. This selected six-event domain does not
claim every interleaving of a separately scheduled Stop and closure, every native
thread schedule, or physical process-death evidence.

The normal Android connection/consent/consumer/lane cohort passes661 cases with
zero failures, errors or skips. The final focused cohort, including the new
720-trace test, passes34. Existing joined private Binder/loopback and original
late/blocked-retirement regressions remain in the ordinary suite. The current
native Rust source is rebuilt through `test-host-offline.py`; all31 selected
media invitation/registration and native registration-client cases pass.
The new methods/callbacks have Kotlin PSI1.9.20 source complexity estimates<=4;
these are not unavailable licensed SonarQube measurements. No new production
function is introduced by this test/audit deliverable. T1241 separately records
native per-function JaCoCo counters for both changed production methods.

## Ownership boundaries examined

- An original Attempt owns its private connection worker and resource sessions.
  The connection lane retains one original active/retiring run and one latest
  pending request. Selection revokes old projections; actual original retirement,
  rather than logical Ended or a deadline, gates replacement admission.
- Consent is bound to exact publication authority and original peer identity.
  Equal DTO/lease values do not carry permission across replacement. Source
  loss/observer retirement clears selection; independent native binding lanes
  still own their actual workers through cleanup.
- Media consumer claims belong to the original session/relay. Closing forbids
  new claims. Terminal receipt requires published closure and every admitted
  claim's actual completion/no-start proof. Sender failure retains one original
  terminal command with bounded retry backoff; it never starts a replacement.
- Actual original connection retirement ends and joins its relay. A late claim
  cannot target the replacement; independent native media obligations retain
  their own sponsors. Unknown native completion is not a reuse witness.
- Host MediaRegistration selects one original discovery candidate and route.
  Source replacement, changed route, withdrawn admission and native/IPC failure
  revoke local authority; none quietly retarget this invitation. Client thread,
  original IPC socket, endpoint descriptors, exact remote Retired proof or original
  broker exit, and actual connector/runtime teardown remain independently owned.
  T1228 retains the original Unregister cleanup exchange after admission failure;
  a second exchange cannot reset the already-started cleanup budget.

Conservation checked within these domains: every admitted original claim is
current or closing until its exact proof; no terminal command precedes its
prerequisites; no old callback gains a newer owner; no metadata completion permits
native reuse. Eventual cleanup is conditional on original native return/death,
accepted original relay delivery (or actual original connection death), and fair
execution of cleanup jobs. Pending or unknown obligations remain retained;
these tests do not establish unconditional leak freedom of native devices.

Fixture resources are bounded:3 media owners/claims and6 generated events per
model trace, with recursion depth6 and720 total traces. Every case closes original
owners in `finally`; binding fixtures cut off queued admission, release original
retirement gates, cancel their scope and join its jobs. Metadata workers and original proof gates are explicitly synthetic; no fixture
result is presented as native Android device authority or physical proof.

## Offline build and limits

Gradle and Robolectric use cached artifacts. Tests run in a separate network
namespace with loopback only and an explicit offline provider directory; release
build also has no external route. No tablet, source GUI, installed service or
installed Android application is touched. Namespace JVM tools report an unwritable
/root user-preference/analytics location; all listed tests and release build
complete, and no application preference or native-support claim derives from it.

`build.json` pins the local release APK path, size and SHA-256. It is unsigned:
default local signing properties are absent. No key is created and nothing is
installed, uploaded, downloaded or pushed. Existing complete-suite/platform
limits (including T835/T1221 and excluded EVDI/Windows work) remain separate.
T1238 and T858/T859/T860 physical acceptance remain unfinished and live checks
stay paused. T1241 is a confirmed software race, not a verified cause of the
previous uninstrumented tablet Stop episode.

</details>
