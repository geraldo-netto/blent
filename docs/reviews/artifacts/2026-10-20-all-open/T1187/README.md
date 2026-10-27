# T1187: terminal decoder metadata/selection epochs

Terminal decoder epochs deny metadata, selection and calibration without wrapping. Wider-integer and held-native-creator tests passed; metadata invalidation alone cannot release original work.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The settings contract reserves u64::MAX as terminal; admitted epochs satisfy
0 <= epoch < MAX. Clearing always discards decoder/selection metadata and
advances toward MAX without wrapping. New decoder_scope/admission queries own
that policy. At terminal, current greetings omit decoder scope/protocol,
reports cannot restore metadata, selector support/matching/calibration deny
admission, and cache fingerprinting cannot start native environment preparation.
A selection reason reports exhaustion and the need for a fresh Blent owner.
No compatibility epoch, alternate schema or replacement cleanup owner is added.

Six permanent normal-suite oracles fail first: counter wrap, revived zero key,
terminal selection/calibration, the independent wider-integer model, terminal
input report/greeting and the actual original native-creator trial. The native
case uses a current valid protocol2 report and a held real blocking thread. After
MAX reset plus stale metadata restoration, the old zero key keeps its work alive
before the fix. The corrected guard settles on one deterministic poll without
using elapsed time as cancellation proof. Its original native group remains
incomplete until the controlled creator releases, is joined and settles. Both
negative and repaired cases release their gate before final assertions; a Drop
sponsor also unblocks native work on partial failure.

Final coverage execution passes198 cases:110 library media/input,76 CLI selector
and12 separate contract-crate cases. Additional permanent cases cover all four
codec families with retained terminal metadata and terminal cache preparation.
The independent u128 oracle checks130 seeds ×80 resets (10,400 transitions).
Geometry/scale edge properties remain at the last admitted epoch (MAX−1), while
new terminal assertions deny MAX explicitly; original boundary assertions stay.

All10 changed production functions meet individual native executable-line
coverage:90–100%, including exact callback-containing settings implementation
rather than the similarly named trait declaration. Missing selection_reason line
is the existing feature-gated in-process diagnostic; no feature validation is
claimed from default counters. Source hashes, commands, raw counters and logs are
retained. Export uses only current lib/bin/contract executables; source-equivalent
canonical paths are combined. Initial separate-test-module inference failed for
the current #[path] contract consumer during formatting; explicit source-relative
module path fixes that current caller too. Final contract crate compiles/runs.
Formatting/whitespace checks pass. Manual Rust source decision upper bounds are
<=9 individually; these are not native SonarQube (T1019). Broader all-features
fixture prerequisites remain T835, and normal example build repair remains
T1189; no full-workspace claim is made.

The original watch settings owner serializes report/configuration updates.
Production assignments to decoder_epoch are initial owner construction or this
clear transition; no current production reset/wrapping caller is retained.
Selections clone immutable key/metadata; they do not own a native release proof.
Scope strings are locally owned and at most20 decimal bytes. Query methods
borrow settings synchronously and retain no callback, queue, borrowed view or
native pointer. Clearing drops only owned metadata; it never releases an encoder,
attachment or capture native lease. Terminal policy derives from the single
counter and adds no duplicated mutable admission state.

Actual old work remains owned by its original NativeScope/Task/Admission and
retirement observer. Revocation can cancel future publication, never certify
creator/child destruction. The native regression drops reporting aliases while
the original creator is held, confirms its exact observation stays incomplete,
then releases it and awaits original settlement. Existing activity/attachment,
receipt, cache and capture ownership tests pass. Conservation: no fresh metadata,
selection or cache admission exists at terminal, and no original obligation is
released merely by numeric invalidation. Conditional cleanup requires actual
native return and scheduler/reaper progress; held controls do not assume those
conditions until explicitly released. Bounded counters/models do not establish
unrestricted leak freedom or physical platform safety. No device/service,
Windows or EVDI backend was changed.

</details>
