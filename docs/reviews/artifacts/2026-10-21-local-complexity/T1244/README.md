# T1244 accepted local cyclomatic verification

The owner accepted suitable local cyclomatic tools while keeping the per-function maximum of 9. Rechecked Kotlin rows matched; separate parser limitations and native coverage obligations remained explicit.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Owner clarification2026-10-21 accepts any suitable local cyclomatic checker;
SonarQube itself is not required. The maximum remains9 for every function.
The existing source checker implements the documented language counts, not
cognitive complexity. Its source/parser limitations remain explicit; this
acceptance does not manufacture vendor certification or a whole-project pass.

The cached Kotlin PSI1.9.20 checker is rerun in a network namespace with no
external connectivity. All541 unique authored functions/methods, initialization
owners and callbacks in T1242/T1245/T1246/T1247/T1248 match their archived rows
exactly. The per-scope count/maximum is99/3,109/6,96/3,154/7 and83/3 respectively.
No row exceeds9. The normal authored-method checker suite passes all9 cases,
including bounded1–13 counts and the9/10 boundary for method kinds. Native
coverage attribution remains independent of these syntax counts.

T1243's archived project Rust counter result uses tree-sitter0.25.2 and the
cached tree-sitter-languages1.10.2 Rust ABI14 grammar. Its two changed functions
score4 and1. The exact source hash still matches the archived input, so the
existing local result remains applicable; this is not claimed as a fresh Rust
parser execution. The separate current whole-project parser limitation T1221
remains unresolved. Macro-expansion opacity is not suppressed or hidden.

`results.json.gz` retains all per-function rows, scope summaries and current
source hashes. Analyzer/shared-ownership source hashes and the checker test log
are retained separately. Original scope reports remain under
`2026-10-21-offline-integration/T1242` through `T1248`; their historical tool labels
are preserved, and their prose now records the changed acceptance requirement.
No production behavior, test exclusion, metric suppression, resource lifetime,
coverage requirement or unsupported-source disposition changes. No downloads,
uploads, physical actions or repository pushes occur in this verification.

</details>
