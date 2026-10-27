# T1247 rendered media actions retain their original owner

Compose media controls use reference-identity keys so captured old actions cannot target a replacement offer. Rendered-control regressions and hosted counters passed; this was not established as the cause of the earlier physical Stop report.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The camera/audio consent domain rejects callbacks carrying an old offer, but
Compose reused the button/switch nodes across different original owners. A
previously captured semantics action could therefore invoke the node's updated
callback after recomposition and act on the new offer. Earlier control tests
called the retained action before recomposition, leaving this boundary untested.

Permanent normal-suite regressions now recompose first, then invoke old Start,
Stop and background-toggle actions for camera and audio. Before repair all12
cross-owner API27/API34 cases fail: old Start grants replacement consent, old
Stop disables replacement consent, and old toggles grant replacement background
access. Four same-owner publication controls pass in that same red run. Both
Start regressions also use distinct, equal-valued owner objects; ordinary value
keys cannot distinguish them. `all-valid-red.log.gz` and both red XML records
retain these failing-before results.

Each consent subtree now uses a Compose key with explicit reference identity.
Different original owners get different nodes and retain their own old handlers;
metadata updates within the same owner keep its controls. The existing domain
authority still validates every offer. No permission, consent, wire or native
resource contract changes. The independent key tests exercise same-owner equality
and hash stability, different equal-valued owners, null and unrelated key types.

All16 original rendered-control regressions and the two key tests pass after
repair. The complete camera/audio consent-control/key cohort passes36 cases,
including existing permission/foreground/restriction/withdrawal checks. The
joined Binder/loopback replacement tests additionally exercise old actions both
before and after the new native speaker starts; their separate fixture readiness
error is corrected under T1248, not represented as a production transport bug.

## Ownership and validation limits

The composition owns one current consent control group per rendered media row.
`MediaControlIdentity` borrows one non-null opaque owner through a strong JVM
reference; it owns no native handle, queue, permission, worker or mutable state.
Its equality checks exact owner identity, so identity-hash collisions cannot
merge owners. Same-owner publication updates preserve the group. Replacement
cuts off node reuse; retained old handlers continue to reference their original
offers and are rejected by domain authority. Composition disposal and eventual
release of queued handler references permit ordinary JVM collection; no new
global retention or cleanup sponsor is introduced.

Native consumers remain independently owned by their original binding/retirement
lanes. UI disappearance, a key replacement, callback rejection or logical Stop
never supplies native release/reuse proof. Existing gate/fault/uncertain-retirement
coverage remains. The tests enumerate bounded original/replacement and same-owner
publication cases; they do not establish every OS input interleaving or
unrestricted leak freedom. Eventual cleanup still assumes framework disposal,
release of pending callback references, fair execution and actual native proof.

Native JaCoCo0.8.15 counters with repository authored-method attribution are
retained in `coverage-functions.json.gz`, with exact production source/class
hashes. All214 scoped production functions/methods across thirteen source files
pass individually at80–100% executable-line coverage. No
aggregate substitute or exclusion is used. Cached Kotlin PSI1.9.20 reports
maximum7 source cyclomatic complexity across the changed production/test scope.
Owner clarification2026-10-21 accepts local cyclomatic tools. T1244 verifies
these source measurements and closes the former native analyzer prerequisite;
no SonarQube certification is claimed.

All validation uses cached providers, Gradle offline and isolated loopback-only
network namespaces. An initial incremental production compile lost metadata for
unchanged top-level Kotlin functions; clean nonincremental recompilation restores
authoritative production/test classes without source workarounds. Current cohort,
native counters and unsigned release-build evidence are retained locally under
`/backups/disk2/blent-work/2026-10-21-offline-integration/T1247/`.
The final broader cohort passes1632 cases with zero failures, errors or skips;
it includes the T1245/T1248 integration prerequisites. `assembleRelease` succeeds
and `build.json.gz` pins the unsigned local APK. Nothing is installed.

No physical test, APK install, installed-service change, transfer or push occurs.
T1238's earlier physical Stop episode remains uninstrumented and unresolved;
this confirmed software bug is not established as its cause. Live checks remain
paused, and native Android/other-device/Windows qualification is not claimed.

</details>
