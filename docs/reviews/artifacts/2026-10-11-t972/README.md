# 2026-10-11-t972 — README

Native input fingerprints now include exempt Rust sources without imposing production coverage thresholds on benchmarks. Real offline import checks preserve source and object attestations.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T972: native Rust input fingerprints are independent of maintained-function threshold ownership. The unchanged permanent snapshot/native-import regression fails before the fix because exempt benchmark counters have no fingerprints, then passes. Invalid/missing/changed/added/removed input properties reject safely; exempt Rust is never parsed as maintained production code. All 90 coverage-tool tests pass. Fresh offline host accessory collection executes 51 tests; its generated flat native-sources.json successfully imports actual config-writes.rs benchmark counters while retaining source/object/report attestations. Benchmark functions remain exempt. The native adapter scope has 14/14 functions passing individually; this focused import qualification does not claim whole-project coverage or physical support. The coverage tools themselves remain exempt development scripts. Changed complexity: 47 functions, maximum 8. No downloads or live device operations.

</details>
