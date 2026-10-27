# 2026-10-11-t951 — README

Microphone permission and Activity ownership now reject stale callbacks and old Activity detach. Independent audio workers retain cleanup and background consent; direct USB audio integration is separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T951: independent original microphone permission-request identity and exact application audio Activity attachment prevent stale SDK results or an older Activity stop from granting, denying or stopping a replacement. MainActivity SDK callbacks retain the original prompt through the shared LocalPermissionLauncher/Prompts contracts from T975. One focused application lifecycle adapter owns foreground observation while microphone/speakers retain independent workers, service identities and actual cleanup lanes. Explicit background sharing survives foreground detach; closed owners cannot revive.

Both permanent bug regressions fail before behavioral fixes, then pass unchanged: actual overlapping MainActivities lose their replacement observer before the fix, and a late permission result starts a replacement even when the identical endpoint object is reused. The independent permission record/optional argument initially added only test identity metadata, without changing result handling; red logs preserve that proof. Normal tests retain original/stale/discarded callbacks, SDK refusal/failure, fresh permission versus grant, 16 bounded permission/lifecycle combinations, invalid directions and independent active audio/background preservation. Earlier boolean-result fixture calls now explicitly supply current original request identity; existing assertions remain. Full normal Android suite passes and all 2523 maintained production methods pass the individual 80% native line gate. Changed complexity has 241 authored methods, maximum 8. Source/native counters, test summary and red evidence retained. No downloads, live tablet tests or host service changes. Direct USB audio remains T860/T859.

</details>
