# T895 — remembered peer presentation

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Restored Android trust now publishes a bounded `Peer` control event containing
Computer ticket/fingerprint, Tablet fingerprint and remembered status. It is
metadata, not TLS proof or a pairing prompt. Unknown peers retain the existing
`Pairing` event without adding redundant queue entries. Runtime cancellation
filters both identity event types; durable trust mutations retain their separate
semantics. IPC remains private to the same APK; event tag 5 is strictly decoded.

Permanent regressions first failed for absent restored-peer metadata and stale
metadata after logical stop. They now pass, including revocation of the exact
remembered peer and no endpoint admission before TLS proof. Codec tests retain
truncation/invalid-boolean checks and 1,024 seeded mutations.

Offline connection suite: 521 tests passed, none skipped. All 84 inventoried
production functions and 123 raw executable JVM methods in the four scoped
files meet 80% individual line coverage; 73 complexity entries are at most 9.
Raw counters, source inventory, test results and red/green logs are retained here.
No physical tablet test or production pairing acceptance is claimed; T855 remains.
