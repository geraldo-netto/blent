# 2026-10-11-t965 — README

The Kotlin metric inventory now includes authored initialization, callbacks and other method kinds. Source-matched native coverage stays unchanged; conservative source rows do not imply runtime non-inlining.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T965: expand authored Kotlin method inventory using shared PSI ownership and independent complexity/native coverage reporters. Initial permanent missing-method fixtures fail before implementation. Additional red regressions isolate anonymous-object ownership and enum argument/static ownership before correction. Final 28 complexity tests and 87 coverage-tool tests pass. The whole owned-source audit includes 20,390 functions/methods/callbacks/initialization owners and none exceed 9; no new excessive source methods remain. Native coverage inventory after shared extraction exactly equals the retained T968 manifest in content; the unchanged full-suite native report still passes all 2,477 Android functions. T968 retains the 1,770-test normal-suite native XML/exec and source hashes. Additional construction/accessor/callback counts extend the pinned recursive source metric; conservative lambda rows do not imply native non-inlining or Sonar certification. No downloads or device testing.

</details>
