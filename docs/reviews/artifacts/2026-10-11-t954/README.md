# T954: audio benchmark source generation responsibilities

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Separate direct packet variant and pipeline track generation from project
orchestration/build/provenance. Preserve every historical/current source branch,
checksum expression, preference fallback and paired originals/source writes.
All three functions score <=9; main previously scored 11.

A controlled offline builder runs original HEAD and refactored generators against
exact current audio sources; every generated source/build/manifest/provenance byte
matches and Gradle dispatch retains --offline. Python compilation also passes.
This checks generation equivalence, not an APK build, performance or device trial.
Benchmark/dev-script coverage exemption remains unchanged.
