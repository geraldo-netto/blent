# T964: native audio read failure survives renderer completion

Audio orchestration now joins its original reader after normal renderer completion so delayed native failures remain visible. Full tests passed, while newly exposed coverage gaps stayed separately recorded.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Controlled permanent regression fails on both simulated APIs before the fix:
renderer completion cancels the original reader while native failure delivery
is still blocked. Actual coordinator dispatch completion synchronizes the assertion;
no guessed sleep schedules the race. The socket adapter's injected factory defaults
to the same production socket, preserving real protocol/Okio/coroutine behavior.

Stream orchestration now joins that original reader after normal renderer end,
preserving its read timeout and actual completion. Cancellation and socket close
remain in finally for local Stop/failure. The regression retains pending native
ownership until failure delivery, then requires the original timeout, exact player
close and idempotent socket close. Existing T719 partial-byte deadline, audibility,
focus and Stop assertions remain unchanged.

Full normal offline suite passes 1,767 tests, without skips/failures. All 11 affected
production functions, including constructor variants, pass native individual line
counters. The 56 reviewed named functions remain <=9; T965 separately tracks the
complexity tool's missing Kotlin method kinds.

Full expanded Android coverage measures 2,483 functions: 2,478 pass. T966 tracks
three null-only singleton initialization rows with no executable native method;
T967/T968 track two uncovered authored default-constructor variants. These are
retained gaps, not exclusions or a claimed whole-project coverage pass. No device
acceptance, download, install/reload or EVDI changes.

</details>
