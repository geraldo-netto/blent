# T843 — literal preprocessor header probes

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Permanent regression failed before the parser adaptation and passes afterward.
Five focused tests cover the real generated-replay producer, both branches,
quoted/angle header sizes 1..4096, byte/line provenance, UTF-8, continuation lines,
nested and in-function guards, untouched comments/strings and malformed inputs.
All fourteen complexity tests and seventy coverage-tool tests pass offline.
The five maintained benchmark files using literal probes now parse completely:
44 functions, all complexity <=9. No file, branch or counter was removed.
The new parser adapter and tests also stay <=9.

The whole-source gate was rerun and still fails on an independent annotation
macro grammar limitation in historical T790 C source. T844 records exact locations
and follow-up; no whole-project pass is claimed. The gate log is retained.
All tools and Kotlin dependencies were cached; no downloads.
