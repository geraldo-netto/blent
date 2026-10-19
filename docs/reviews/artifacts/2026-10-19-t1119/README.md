# T1119 native cadence checkpoint

Permanent normal-suite regressions failed before the production change: a
one-second request returned compatible 200 ms, and a valid 500 ms request waited
to 1500 ms instead of the independently required 1200 ms check. Both now pass;
500/1000 ms boundary, fresh-frame, expiry/removal and stop checks remain. Two
deliberate faults restoring 500-only validation or removing the deadline cap
are rejected. Each original source file was restored in finally before final
counter collection. The sandbox LSan failure is retained in green-first.log;
sanitizers were rerun unchanged outside ptrace containment.

Final native normal suites: 63 helper + 1 separate-module + 1 frame-retirement
tests, four test threads, zero failures/ignored. GCC/gcov 13.3.0 executable-line
counters measure every definition independently, inventoried with Clang 18.1.3:
16 production functions, 15 at 100%, writer_run at 88.235%. Raw profiles,
notes, compile commands, JSON reports and fixed native input hashes are retained.
One unused instrumented compilation emits gcov's missing-data warning; no
function is omitted or credited without measured native execution.

Existing writer benchmark ownership/compile regressions (2), independent padded
cookie fixture (1), architecture rules (5) and Arch/release build fixtures (2)
pass. Build fixtures use local command stubs; this is not a release build or
package/runtime acceptance. Original PKGBUILD build() receives native Bash
5.2.21 events on all five executable command lines: 100%. build-release.sh's
changed command is top-level, with no new/changed shell function. Candidate
writer interval experiments still patch only their copied compatibility default.

Source estimates cover 111 C/Rust/Python/Shell functions, maximum 9. Native
SonarQube remains unavailable under T1019. Complexity JSON distinguishes source
estimates from native analysis. No suppression, new exclusion or weakened
invalid-input oracle is introduced: validator invalid result is now 0 (no
advice), while writer fallback remains 200 ms; invalid corpus and input bounds
are retained, and writer fallback is independently checked.

This checkpoint does not enable one-second mode through configuration. T1120
retains replacement-source admission work; T1121 retains exact-record NUL
validation. T1115 retains joined original-viewer/recipe/decoder/ACK policy;
T1116 retains shared/native encoder qualification. No live EVDI/tablet,
battery, GPU, HEVC, physical presentation or full runtime-support claim.
