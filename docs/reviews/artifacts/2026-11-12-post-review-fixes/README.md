# Verified fixes for the post-fix review

T1370–T1375 each have a separate fix commit and a permanent regression that
failed against the prior behavior. T1377 was discovered during T1374: Wi-Fi off
read one saved address but could clear another after a concurrent update. Its
separate regression failed before the transaction-based fix and passed after it.
Resolved rows were removed from TODO.md. Existing blocked/declined rows are
unchanged; T1376 records the new Windows CLI entry's missing native counters.

| Item | Result and retained regression |
| --- | --- |
| T1370 | Exact source/AppImage autostart publication; `scripts/tests/test_autostart_publication.py` |
| T1371 | Exact source/AppImage service publication and failed-stage cleanup; `scripts/tests/test_service_publication.py` |
| T1372 | Exact AppImage image/CLI/GUI executable publication; `scripts/tests/test_executable_publication.py` |
| T1373 | Exact AppImage desktop publication and failed-stage cleanup; `scripts/tests/test_appimage_desktop_publication.py` |
| T1374 | Typed connected/disconnected/no-saved-address/ADB-unavailable results; failed cleanup admission returns an error after configuration clearing. Shared tests in `host/src/wifi/tests/off.rs`, native CLI contention in `host/tests/wifi_setup.rs`. |
| T1375 | USB model/manufacturer projection joins the exact connection ID; descriptive names outrank generic identity labels without changing identity/presence/assignment authority. GUI and common workspace regressions retain both assertions. |
| T1377 | Capture and clear the address under the same storage transaction, before native cleanup admission. `host/src/wifi/tests/atomic.rs` covers stale snapshots, subsequent saves, missing edit and failed commit. |

Each item directory retains failing-before and passing-after logs and individual
function measurements. Final counters, source hashes, tool versions and command
results are in [final/verification.json](final/verification.json). All **25 of 25
measured changed Linux/shared production functions** reach 80% executable-line
coverage and complexity at most 9. The changed native Windows CLI entry remains
unmeasured under T1376; it is not included in that passing count.

Final installer validation passes 49 of 49 selected tests. Common and GUI suites
pass 430 of 430 and 192 of 192 tests. Before the atomic-address follow-up, the
host library passed 1,062 of 1,065 registered tests with three existing ignored
cases. The final follow-up passes all 35 selected Wi-Fi library tests, both
process-exit tests and all four native CLI tests. Child test-process summaries
are not counted as additional independent cases.

Formatting and CI-scope Clippy pass. The whole-project complexity check measured
28,884 functions with none above 9. Windows all-features workspace/test
cross-compilation passes; its existing Linux-only test-variant dead-code warning
remains visible in the log. No native Windows or physical-tablet acceptance is
claimed. Per-function evidence uses native Bash traces or LLVM counters with the
project's source inventory; it is not an aggregate coverage substitution.

Ownership review: AppImage publication admits one staged file in the destination
directory, performs synchronous rendering/mode-setting, then either renames to
the exact path or removes its staging on exit. Directory and symlink-directory
rejections preserve existing contents. Wi-Fi errors do not release already
admitted native cleanup; the existing process-exit fence remains intact. The
transaction handoff retains one owned address and admits no ADB work before a
successful commit. GUI name projection borrows one validated immutable snapshot
and creates a bounded label; names grant no pairing or resource authority.

These bounded regressions and source checks are not an unrestricted native
lifetime proof. Existing display-route ownership decisions and physical/native
acceptance blockers remain in TODO.md. No changes were pushed or installed.
