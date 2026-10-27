# 2026-10-11-t982 — README

Camera and audio now share original-owner consent contracts while retaining resource-specific policy. Hosted tests preserve explicit local intent; native audio composition and physical acceptance remain separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T982 extracts one generic original-owner Android media consent policy and observation through independent MediaAccessOffer/MediaAccessAuthority/MediaLocalConsent/MediaConsentView<O>/MediaAccessControls<O> contracts. Current camera callers and all permanent camera regressions migrate together; there are no duplicate policy implementations or compatibility wrappers. Camera lens metadata remains camera-owned. AudioAccessOffer validates microphone/speaker metadata and prevents speaker offers from requesting microphone SDK permission. No transport, credential, worker or OS handle enters these contracts.

This is a behavior-preserving camera extraction and new audio contract qualification, not a behavioral bug fix; no artificial red test was introduced. All retained camera stale/forged/cancelled/synchronous permission, original publication, background, native capture selection, controls and production Activity fixtures remain. New permanent audio properties cover both directions' explicit local Start and independent background choice, offer-to-admission continuity, equal metadata across replacement owners, stale/hidden/replaced SDK results, denial/fresh permission, unavailable launchers, withdrawal/failing sources, direction isolation and cancellation/late publication. Bounded properties test all 256 direction byte values plus negative/integer extremes and all offer flags; 64 local lifecycle combinations retain only explicit current background consent. Logical UI/source closure never claims native media or endpoint retirement.

Fresh offline full Android suite: 1,814 tests, zero failures/errors/skips. Native JaCoCo gate passes 2,524/2,524 maintained production methods individually; no missing counter or threshold weakening. Authored constructor records, full XML/exec counters, pre-build source inventory, source fingerprints and logs retained. Changed-source complexity: 342 authored functions, maximum 9. Camera production uses the shared policy; native leased audio routing, worker/consumer cleanup and production controls remain T978/T979/T859, with physical acceptance T860/T859/T846. Production capability remains display/camera only. No downloads, installs, live tablet trials or service interruption. Work stops after this item at the owner's request for network setup changes.

</details>
