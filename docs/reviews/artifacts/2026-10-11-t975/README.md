# 2026-10-11-t975 — README

One independent permission mediator now serves camera and audio callers without duplicate implementations. Existing stale-result and cancellation regressions validate the behavior-preserving extraction.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T975: one domain-independent local-permission contract and mediator now serve original attachment/action identity and the actual SDK request slot. Camera policy remains camera-owned; current callers and retained tests use LocalPermissionAction/Attachment/Prompt/Prompts/Launcher directly, with no duplicate audio implementation or compatibility aliases. This is a behavior-preserving contract extraction, so existing permanent stale/synchronous/cancelled/throwing/denied SDK-result regressions provide its coverage; no artificial red behavioral test was added. Full normal Android suite passes, all 2507 maintained production methods pass individual native 80% line coverage, and changed complexity is 259 authored methods, maximum 9. Raw JaCoCo/source evidence retained. Audio integration and original Activity ownership remain T951. No downloads or live device operations.

</details>
