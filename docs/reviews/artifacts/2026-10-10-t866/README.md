# T866 worker fault fixture complexity

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Extracted fault injection from the worker ownership regression; all four failure
phases and every ownership, surface and retirement assertion are retained.
The original test measured cyclomatic complexity 10 before extraction and 3 after;
the helper measures 8 and all 23 functions in the file are at most 8.

Offline `:app:testDebugUnitTest --tests com.blent.MediaWorkerServiceTest` passed
all 10 tests on the configured API 27/34 Robolectric targets. Test-only structural
change; no production behavior changed and no new artificial regression needed.
The historical T856 complexity artifact retains the original finding.
