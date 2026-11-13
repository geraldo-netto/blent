# T1383 — shared physical identity validation

The permanent Linux alias-selection regression fails before the fix and passes
afterward. Both adapters use the independent common contract: strict UTF-8,
1–1024 trimmed bytes, no embedded controls, and no unknown sentinel. Invalid
observations remain distinct; valid Unicode and maximum-length aliases deduplicate.
The bounded common regression covers C0/C1 controls and length boundaries.

The selected portable USB suite passes 35/35 cases. All three changed production
functions exceed 80% executable-line coverage; complexities are 1, 1 and 2.
Tools: cargo-llvm-cov 0.8.7, project syn 2.0.119 complexity frontend.
The pure parser borrows response storage only until its caller copies the result;
no native ownership, query cancellation or endpoint reuse boundary changes.
Linux commands use private shell fixtures; no physical device is mutated.
