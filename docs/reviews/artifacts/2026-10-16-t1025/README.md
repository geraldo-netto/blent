# T1025: resume snapshots satisfy fallback pacing

Resume snapshots now advance the existing fallback deadline, avoiding one redundant grab while retaining lost-update recovery. No physical power or CPU gain is claimed.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The capture context owns the fallback deadline alongside request pacing. A
successful resume snapshot advances that original deadline. The first resumed
tick therefore waits for the configured request instead of immediately grabbing
another fallback image. The existing watchdog still recovers a lost update after
250 ms and advances the same fallback deadline; recovery policy is unchanged.

The permanent normal-suite regression fails before the fix with
`T1025: resume snapshot must satisfy fallback deadline`. A mock clock then
verifies the same test passes: the initial resumed tick adds no grab, the next
configured request runs, the lost-update watchdog still recovers, and another
resume after a long pause again avoids the redundant grab. Only the internal
tick call signature changes with ownership; assertions are retained.

Fresh locked offline native collection passes 75 tests across the helper,
conversion, independent module linking, frame retirement and Rust/native
activity integration. All 239 native production functions exceed the individual
80% line threshold. Native GCC reports, compiler invocations, source/function
manifest and hashes of every native evidence file are retained here. Original
notes/counters remain at `/tmp/blent-t1025-evidence/coverage/native` and their
hashes appear in `native-files.json.gz`; no counters from older production sources
contribute. Existing sanitizer and race checks remain enabled.

The source audit covers 68 C production/test functions, maximum complexity 9,
using unchanged repository Sonar-rule definitions and cached pinned parsers.
Native SonarQube remains unavailable under T1019. Tool versions are included.
There was no network transfer or live device/desktop change.

```sh
cargo test --offline --locked -p blent --test evdi_helper t1025
python3 /tmp/blent-t1025-evidence/coverage.py coverage
```

The coverage script uses the unchanged setup preserved with T1021 under
`/tmp/blent-t1021-evidence/c_setup.py`. This fixes one redundant native grab per
qualifying resume; it does not establish a physical battery or CPU improvement.

</details>
