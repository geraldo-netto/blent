# T1075: report collection without unrelated parsers

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Pure report imports/collection now require only their own dependencies. Source
discovery, snapshot and build-source validation still load the real inventory
and its parsers when used; missing dependencies remain errors. No source checks
are skipped. The collection has the caller-owned bounded manifest/report lifetime
and adds no native allocation, callback, queue or asynchronous cleanup owner.

Both permanent import-isolation regressions fail before lazy imports, then pass.
The independent source-operation control still rejects missing inventory. All
25 import/driver/native-Kotlin/report/invocation tests pass offline with Python
3.12.3. One preliminary command named two nonexistent test modules; its log is
retained and is not a passing collection. Source AST audit of 17 functions has
maximum estimate 9; native SonarQube remains T1019. Snapshot/CLI integration needs
the existing tree-sitter dependencies, unavailable to this interpreter; no new
full inventory/CLI certification, exclusion or download is claimed.
