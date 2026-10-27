# T991: shared original USB media registration and connection adapters

Shared Linux media adapters validate exact bootstrap credentials before registration. Camera behavior and retirement remain covered; audio PCM integration is a separate step.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Portable `media_connection::Bridge<P>` shares exact invitation, original monitor,
bounded close and retained Drop cleanup for typed camera/audio profiles. Native
Linux discovery/process/descriptor setup is isolated in its boundary adapter;
independent contracts and MediaRegistration contain no OS handle types. Camera
retains its typed request projection and unchanged framing/feedback behavior.
Shared request/credential validation precedes native registration ownership.

A permanent constructor regression failed: bootstrap and original registration
credentials could differ. The same test passes after exact schema/credential
validation before native setup. Bounded tests cover all token lengths 0–128,
all byte values, multibyte UTF-8, every credential-byte mismatch, resource/profile
mismatch and invalid audio buffer/processing values. Camera boundary error
semantics remain unchanged. Existing camera ownership tests remain, with their
original assertions, in the shared module. Typed audio tests preserve exact
profile/resource, replay rejection, held close and retained actual Drop proof.

Private real mutual-TLS/broker camera tests still exercise the descriptor path,
unchanged camera bootstrap/framing/feedback and delayed remote retirement.
Native tests are guarded to the implemented Linux adapter; shared domain remains
portable. Linux evidence does not qualify Windows runtime/counters (T902).
This extraction does not enable audio; PCM composition is T992, controls T979
and postponed physical acceptance T860. T993 records the newly reviewed internal
audio helper raw-token slicing/profile-validation gap before PCM integration.

Final fresh offline accessory suite: 58 host + 17 common cases. Default suite:
55 host + 17 common cases. All 38 production functions in recorded affected
scopes pass individual 80% native executable-line coverage; no missing counters
or weakened thresholds. Changed Rust complexity: 54 functions, maximum 6.
Both collections use identical final source inventories, exclusively owned fresh
target trees and matching source/native-object/report/input fingerprints.
LCOV, source inventories, per-function counters, logs and attestations retained
compressed. Broad native-audio SDK prerequisites remain T835; no all-features,
whole-project or foreign-platform pass is claimed. No downloads or live trials.

Build-input attestation describes the tested working tree, including the
pre-existing uncommitted TLS fixture. Those unrelated edits remain unstaged;
this commit neither changes nor includes them.

</details>
