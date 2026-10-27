# T1047: retry encoder retirement after cancellation

Encoder retirement now consumes a reader result once and can retry waiting for original idle cleanup after cancellation. The permanent regression reproduces the earlier consumed-task panic.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

`EncoderTask::retire` consumes the reader task result before waiting for its
original idle controller cleanup. Canceling during the second wait leaves the
reader joined; retrying `finish(false)` previously polled the consumed task and
panicked at `Task::poll`'s missing handle. The permanent normal-suite regression
holds actual original idle dependency cleanup, proves cancellation occurs after
reader join, and retries retirement twice without weakening the held-proof check.
The final oracle fails before the fix with the original `Option::unwrap()` panic.

`Task::is_joined` observes its existing original handle state, with no copied
completion flag. Encoder retirement consumes that result at most once, then still
waits for actual original idle cleanup. Joined result observation is explicitly
not native resource completion; existing error/panic/cleanup tests remain intact.
No timeout, retirement proof, task Future contract or native ownership is weakened.

Source audit covers three functions at maximum complexity 7 with unchanged
repository Sonar-rule counting and cached tree-sitter-rust 0.24.2/tree-sitter 0.25.2.
Native SonarQube remains unavailable under T1019. Native counters and the normal
capture suite are retained alongside this note. No downloads, installation, push
or physical capture/power transitions occurred.

Final native normal capture suite passes 160 tests with two existing ignores.
Both maintained production functions measure 100% executable-line coverage
individually. Raw LCOV, source hashes and native-object attestations are retained.

</details>
