# T1073: original policy observer failure boundary

Policy-observer failures withdraw only their original foreground claim and cannot stop a replacement. Application-owned native cleanup remains independent of observer completion; hosted fault tests passed.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Ordinary policy-flow/publish failures settle only their original foreground
producer. Canceled inputs withdraw their exact claim before cancellation propagates;
completed flows retain ownership until explicit stop. Late old failure never stops
a replacement. Cleanup/reporter faults stay contained and retain distinct diagnostic
causes without self-suppression. Actual queued/native cleanup remains sponsored by
the Application/runtime, independently of original observer-job completion. An
explicit collector method measures delivery separately from the coroutine boundary.

Permanent current/late-error regressions fail on SDK 27/34 before the boundary;
canceled-input regressions also fail on both SDKs before withdrawal. Further cases
cover publish failure, independent stop/reporter errors, cancellation cleanup reports
and a shared failure object. Actual original observer jobs settle, old callbacks
cannot retarget new claims, and fake native retirement counts remain conserved.
Existing completed-flow, rejection and T807/T1064 Activity cleanup assertions remain.
Normal withContext exception recovery may produce a causal copy; the late-error
oracle retains the exact original or its direct cause, rather than requiring an
unsupported wrapper identity. The preliminary diagnostic collection is retained.

Final clean offline suite: **2,193 tests, zero failures/errors/skips**. All **10**
maintained authored production functions/callbacks independently measure **100%**
native JaCoCo 0.8.15 line coverage. Frozen source/vector and compiled class hashes,
raw execution/XML counters and pinned Kotlin PSI 1.9.20 inventories are retained;
no new exclusions. Source audit covers 73 production/test functions, maximum estimate
6; native SonarQube remains T1019. Twelve architecture/contract/portability checks
pass. Existing compiler warnings remain T1072. No network, installation, push or
live device interruption occurred. Activity/window source integration remains T1061.

</details>
