# T1206: remove the obsolete HEVC-only shell inventory

Shell inventory requires current version 2 throughout the receiver, worker, formatter and doctor. Old-version and boundary tests passed while unavailable current evidence remains explicitly unknown.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The current doctor requests shell inventory version2. Android no longer emits
HEVC-only V1, coerces invalid versions to1, or admits shell tag1 at the worker
JSON boundary. The receiver rejects missing/obsolete/invalid versions before
creating query work. RemoteMetadata and direct inventory formatting validate
version2 independently. Current all-family V2 output and internal display
metadata tag0 remain; unavailable current inventory still reports unknown.
Doctor accepts only current V2 evidence, including for HEVC. There is no old
output formatter, alias, migration or compatibility parser.

Permanent regressions first fail: host accepts V1 as HEVC evidence; Android
formatter, remote query, ordered broadcast and worker decode admit old inputs.
All same central rejection oracles now pass. Android covers missing tags,
-8..8 and integer extrema, invalid remote requests and zero receiver query calls.
Host covers all four families, five old inventory values and versions0..32 plus
u32::MAX. Positive hardware/software/depth/absent/unknown, query isolation,
held-worker cancellation and native cleanup assertions are retained. Positive
fixtures use V2 instead of old implicit/default V1. The initial combined Android
run had two failures because T808's positive broadcast lacked its version extra;
that fixture now explicitly requests2 with all latency/retirement checks intact.
Raw failure evidence is retained. The first Rust command accidentally selected
only the library and ran zero cases; its log is retained separately. The actual
binary regression failed on Some(hw10) vs None before repair.

Verification: all48 doctor binary cases pass normally and instrumented. All64
Android cases across five classes pass on cached API27/34 (existing rich-profile
cases also include29/30). Commands use offline/locked Cargo and cached offline
Gradle init; no download or device action. Formatting/whitespace checks pass.
Native source-matched LLVM counters for inventory are9/9 (100%). JaCoCo0.8.15
reports100% for every changed Android production method and callback, including
default bridges; all16 retained inventory/receiver/changed-method variants are
83.33–100% individually (the83.33% decoderEntry is unchanged). Raw LCOV/JaCoCo,
source hashes, method counters and test XMLs are retained. Rustc1.98.1,
cargo-llvm-cov0.8.7. Cached Kotlin PSI1.9.20 source estimates for changed methods
are<=5, including test callbacks; Rust authored inventory/tests estimates<=4.
These are not native SonarQube measurements; missing measurement remains T1019.

Android validates before owner/query admission and keeps one authoritative
WorkerQueries native owner. Valid V2 requests retain existing original worker
execution/cancellation/retirement, pending broadcast finish in finally, and
metadata lane ownership. New guards add no retained state, handle or callback.
The current report has four codec families; existing worker JSON32KiB and
worker result limits remain authoritative. Doctor inventory borrows immutable
command text and returns a bounded-lifetime substring; its scan allocates no
new state and is bounded by existing command output budgets. Fixture scopes
release held workers/timers, join original cleanup, and unregister receivers.
No numeric completion or unknown native result grants reuse. Cleanup requires
actual worker/process exit and scheduler progress; hosted cases do not establish
physical device acceptance or unrestricted native safety. T1199 separately owns
scoped display decoder negotiation; T1192's remaining compatibility audit stays
open. Windows/EVDI, services, APK installation and network transfers are untouched.

</details>
