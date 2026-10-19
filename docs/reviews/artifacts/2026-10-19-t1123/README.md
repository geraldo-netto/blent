# T1123 native retirement observation clock

The normal permanent regression retains the original idle cleanup behind its
actual native file-effect mutex, starts observing its original Completion, and
advances paused media time by six seconds. That must not spend the existing
five-second native observation budget. The original test helper fails before
the fix, despite its cleanup correctly remaining owned and funded.

The helper now uses std::time::Instant, cooperatively yields, and succeeds only
on the same original Completion. Its bound remains five seconds; the other
native-effect helper retains its three-second bound through shared polling.
No production deadline or behavior changes. Deadline failure remains a test
failure; it does not declare native retirement, release capacity or kill work.

The new fixture's mutex sponsor waits for its owning release channel rather
than elapsed time. Both normal and deliberate clock-fault paths release it,
join the original holding thread, and observe actual original cleanup before
asserting outcomes. Existing held-file fixtures preserve their 400 ms protocol
through the same setup helper. No test is skipped or excluded.

All 40 normal idle tests pass. Restoring the old media-clock timeout in the
current fixture fails again; finally restores exact source before the full
suite passes. Test-only changes need no production coverage counter. All32
Rust test/helper functions have source estimates <=5 using cached
 tree-sitter-rust0.24.2/tree-sitter0.25.2 and repository rules; this is not native
SonarQube evidence (T1019). No native device or network operation.
