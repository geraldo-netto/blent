# T852 persisted pairing composition

Trust now composes initial pairing and post-TLS admission on both sides. Saved
approval selects Approved instead of Pending, but a fresh exact stock TLS proof is
still required. Matching authentication failures and failed remembered persistence
cancel the attempt; stale callbacks cannot cancel another current attempt. Failed
storage never silently downgrades to an unremembered prompt. Existing explicit
native-retirement barriers remain required before replacement.

Linux integration retains the identity-store owner, loads real persisted keys and
trust, performs stock mutual TLS, reopens both stores with unchanged certificates,
requires a fresh TLS exchange on saved approval, revokes, then requires approval
again after restart. Wrong TLS certificates never create saved approval. Android's
real stock TLS adapter against the Rust fixture performs persisted remember,
restart/reconnect, revoke and unremembered admission at Robolectric API27/34; wrong
pins/prefaces never reach persisted admission. Existing AndroidKeyStore and TLS
implementations are unchanged; no key export or provider replacement was added.

Offline validation: 233 common tests (four new T852 admission tests, including 256
stale-epoch cases), 30 host TLS/storage tests (two new persisted-TLS tests), 19 focused
Android tests and four Rust/Android TLS interop tests pass. Android includes 256
bounded invalid/stale inputs. All 28 scoped Rust and eight inventoried Kotlin
functions pass the 80% gate; all 10 maintained ConnectionTrust native method counters,
including constructor bodies, are 100%. Scoped cyclomatic complexity is at most 7.

This completes persisted pairing composition, not the production USB application.
T853/T854 still own broker and Android worker integration, T855 the production UI,
T857 display/touch and T858 webcam. Native Android16 filesystem evidence remains
under T868; current tests do not newly qualify AndroidKeyStore/TLS on older hardware
(T863). No downloads, installs, restarts or live key/trust changes occurred.
