# T1194: remove obsolete persisted cache preference

The unused persisted profile_cache preference was removed from current serialization and GUI naming. Targeted tests passed; an unrelated instrumented retirement failure remained separately tracked.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`FileConfig.profile_cache` and its default are removed. Automatic tuning already
uses durable environment-keyed records; the old boolean had no active production
reader. Current parsing ignores unknown keys generically; current serialization
contains only the current schema, including during real locked edit transactions.
No named obsolete-field migration, wrapper or fallback replaces the removed field.
The active GUI method is named `setting_calibration`; button behavior is retained.

Three permanent normal-suite tests first fail on obsolete schema retention, then
pass with the field absent. Retain the T480 schema check under its updated current
contract, eight bounded unknown value types, both boolean inputs through real
locked transactions, independent settings and durable-cache checks. All390
shared tests pass normally. The21 targeted workspace cases pass normally and with
native instrumentation:11 host cache/tuning,8 shared merge/schema and2 GUI
persistence/isolation. T703 still merges an independent current adaptive-idle edit;
T537 still proves original external user configuration bytes are untouched.

Native LLVM executable-line counters meet80% individually for FileConfig::default
and the renamed GUI method; exact totals, raw LCOV, commands and source hashes are
retained. Authored-source cyclomatic estimates: default1, unchanged GUI button2,
new/changed test methods <=5. These are local estimates; native SonarQube remains
T1019. No generated Serde scaffolding is treated as a separately maintained method.

The initial full instrumented suite passed389 and failed the unrelated T814 child
retirement phase assertion (Incomplete versus Exited); its original failure log
is retained, and T1197 tracks causal investigation. The targeted coverage pass is
not presented as a full native-suite pass. T1195 separately owns the pre-existing
GUI recalibration counter wrap; this schema cleanup does not resolve it.

The configuration owns its Strings and nested preferences; removing a boolean
adds no allocation or alias. Serde builds locally owned current records. ConfigStore
retains its existing locked load/merge/atomic-save owner and original lock/tempfile
cleanup on success/error. Tests scope temporary directories and store handles
locally and wait for original GUI work before release. The renamed UI method only
borrows App/UI for a frame; no new job, native resource, callback, queue or reuse
boundary is added. Actual tuning leases and manual-profile precedence remain
unchanged. Hosted tests do not establish native device acceptance or unrestricted
leak freedom. No physical device/service, Windows or EVDI backend was changed.

</details>
