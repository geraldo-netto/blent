# T1046: retained-monitor native display quiescence

`CaptureManager::quiesce_display` stops the original encoder child and reader,
coordinates their retirement with the original helper pause acknowledgement, and
waits for the exact original encoder generation's native creators/proofs. Only
then can it clear codec headers, request a fresh IDR and rotate the FIFO. The
original helper child, monitor/card, stdout owner and control endpoint remain.
Stopping and acknowledged writer quiescence proceed concurrently, avoiding a
circular wait in which pause waits for a FIFO writer while the caller has not
yet stopped its reader. Actual stalled native ownership still retains quarantine.

Cancellation retains the original pending control request and native quarantine.
Retries use the original joined-result state from T1047. Native join observation
continues after the normal bounded finish grace; the retained transaction is
interruptible by dropping its future, but elapsed time never supplies completion.
An actual blocking-reader regression reproduced the draft abandoning join progress
after grace expiry. The transaction now awaits the original reader retirement
workflow without that abandonment, then still checks actual generation proofs.
Malformed/foreign ACKs reject quiescence and preserve the FIFO rather than claiming
success. Unstarted owners quiesce safely; uncontrolled helpers explicitly reject
the unsupported retained-pause operation.

Seven permanent normal-suite regressions cover sixteen same-helper generation
cycles with fresh FIFO/header/IDR state, held actual joined-reader cleanup,
canceled original ACK/retry/resume, canceled startup native creators, foreign ACK,
unstarted/uncontrolled compositions and a blocking reader that finishes after
stop grace without canceling the transaction. Initial shutdown-based assertions
fail because the helper disappears and original reader cleanup is ignored. The
additional grace-progress oracle fails with its exact timeout before correction;
the final unchanged assertions pass. Native subprocesses/socketpairs are harmless
private fixtures, with no physical EVDI/tablet/power access.

The source audit covers nineteen production/test functions, maximum complexity 7,
with unchanged repository Sonar-rule counting and cached tree-sitter-rust 0.24.2
and tree-sitter 0.25.2. Native SonarQube remains T1019. Seven architecture checks
and the normal production compile pass. CLI and installed SDK evidence includes
independent fresh target trees, source/native-object attestations, raw LCOV and
individual function counters. SDK uses installed Freedesktop 25.08,
libavcodec 61.19.101/libavutil 59.39.100, with networking disabled. Broad optional
suite prerequisites remain T835. T1048 separately repairs the optional encoder
test caller exposed by the first SDK build; that failed log is retained there.

This operation is available to supervision, but daemon policy does not enable
controlled helper startup until T1015 integration. Controlled helpers start
inactive; enabling before a current guarded resume would stall capture. Joined
source-driven startup/running pause, current attachment validation and physical
acceptance remain T1015/T914. No downloads, installation or push occurred.

Final normal CLI daemon suite: 516 passed, four existing ignores. Installed SDK:
81 capture plus nine encoder-worker tests passed, with no ignores. All five scoped
production functions measure 100% native executable-line coverage individually
in each configuration. T1048 independently passes nine native encoder regressions.
