# T1141 original authority-job rejection

A full or closed authority queue returns the exact original job in an owned box. Existing lifecycle tests and scoped Clippy passed; native thread retirement remains independently required.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Worker::submit returns Box<AuthorityJob> on a full or closed queue. Successful
jobs still enter the same one-job channel without an extra allocation. Each
rejection moves the exact original Pairing capability into one caller-owned
container; no clone, new authorization, escaping borrow or retaining cycle is
introduced. The caller can retry, run or destroy that original job. Worker close,
queued draining, persistence and native thread admission/join are unchanged.

The existing normal tests pass7/7, including original full/closed rejection,
successful use of the returned original job, initialization/storage failure,
stalled persistence, actual thread retirement and capacity retention. These
exercise the added owned container without mirroring its representation.
Native cleanup remains sponsored by the original worker admission until join;
a blocked store stays funded and eventual cleanup assumes the store returns
and its thread receives execution. Native USB/process-loss qualification remains
separate T862; no new crash/OOM guarantee is inferred from boxing.

Pinned source audit stays <=9 (not actual SonarQube; T1019). Fresh native
counter results follow. No Windows/EVDI edit, physical-device action, install
or network transfer.

Fresh source/compiled-object-attested LLVM collection passes7 tests. All8
production functions pass individually >=80% executable lines, with no missing
counters or new exclusions. Source audit maximum7; Cargo formatting passes.
`cargo clippy --offline --locked -p blent --lib --no-deps --features
accessory-transport -- -D warnings` passes. The broader dependency check still
stops at excluded EVDI T1131; no whole-workspace lint pass is claimed.
Raw evidence: `/backups/disk2/blent-work/2026-10-19-open-items/t1141/native`.

</details>
