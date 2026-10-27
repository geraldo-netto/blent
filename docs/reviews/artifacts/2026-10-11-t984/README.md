# 2026-10-11-t984 — README

Leased speaker ownership now requires local consent and independent endpoint/consumer retirement. Private transport fixtures preserve bootstrap bytes; production enablement and acoustic acceptance are separate.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

T984 implements Android locally leased speaker ownership behind an explicit original resource authority. Pending offers require local Start; foreground/background and current choice remain independent of pairing and metadata. Speaker playback invokes no microphone SDK prompt. LocalAudioOffer/Route validate lease/profile/port boundaries. Production capability remains display/camera only until T985/T978 joins controls and playback; microphone remains unavailable here until T859.

Camera and speakers share LocalMediaResource for lazy endpoint ownership, original source authority, possible external consumer exposure and actual cleanup barriers. Existing camera consent stays in its independent policy adapter. Logical closure/Stop/expiry cancels admission immediately but releases capacity only after native endpoint/pump and exposed consumer retirement. Uncertain proof publication quarantines without retry or false release; siblings remain independent. Speaker uses the existing bounded bidirectional pump, preserving bootstrap/clock/PCM wire bytes without Binder transport.

Twelve permanent cases cover all 32 background/local-choice combinations, original offer/admission, equal metadata and stale/seeded foreign proofs, both cleanup orders, pending/denied/unpublished routes, original expiry/revocation, foreground/background/permission loss, endpoint failure, uncertain native proof, invalid processing/buffer/ports/lease/clocks and explicit unsupported composition. Real private management/mux tests preserve a fragmented 76-byte bootstrap and 2,064-byte reply/clocked-frame extent through partial writes/flush, then require actual original consumer/endpoint proof before protocol REVOKED. Private joined controls preserve late original completion after Stop. Existing camera/other-channel stall, I/O boundary and worker failure fixtures remain. These in-process synthetic fixtures do not establish physical AudioTrack/provider, acoustic or debugging-off acceptance; T978/T860 retain that work.

Fresh offline full Android suite: 1,841 tests, zero failures/errors/skips. All 2,581 maintained production methods pass individual native JaCoCo thresholds. Full matching pre-build inventory, native XML/exec, counters, source hashes, authored constructor records and logs retained. Changed Kotlin complexity: 203 functions, maximum 8. No downloads, installs, service disruption or live tablet tests.

</details>
