# T1074: authored inline counter gaps

Coverage reports now expose missing authored inline-body counters instead of accepting factory-only coverage. Retained false-pass evidence fails the stricter checker; ambiguous compiler mappings still require independent evidence.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Filtered native inline classes are associated with their original compiled
method owner. If an authored lambda lacks separate counters and that owner has
a filtered inline class, the authored body and enclosing method stay visibly
unmeasured. Native factory coverage alone cannot pass. Diagnostic variants
retain original visible counters; the checker invents no execution counts.

Permanent regressions fail before the checker change for same-line/nested
transforms and the actual retained T1069 XML/inventory. An additional partial-
class control fails before its refinement. All eight new tests now pass, with
33 report/import/driver/callback/invocation checks passing offline. Owned callback
counters resolve ambiguity; library-only wrappers without authored callbacks
retain enclosing-function measurement. Bounded foreign prefixes and malformed
identities are checked. No new exclusions or weakened native assertions.

The retained T1069 factory-only pass is rejected (both inventoried rows fail).
All 39 genuinely measured T1061 functions/callbacks still pass; that complete
native report has no filtered-inline owner associations. Raw historical XML,
class/SMAP and execution evidence remains in its original T1069 artifact.
This conservative source/native-owner check does not reconstruct filtered
instructions or prove every possible compiler/SMAP mapping. Ambiguity requires
measurably owned code or independent native evidence, not a wrapper-only claim.

Report/XML/manifest data remains scoped to the existing bounded report read and
caller-owned collection lifetime. The added pure projections allocate no native
resource, callback or asynchronous cleanup owner. Python 3.12.3 source AST audit
covers 40 functions, maximum estimate 9; native SonarQube remains T1019. Existing
nonessential development-tool coverage exemption remains; tests verify the gate's
behavior rather than introducing exclusions. No download or network occurred.

</details>
