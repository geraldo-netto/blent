# T868 persisted authenticated peer trust

Portable Rust/Android policies store only current authenticated remembered approval.
Stored pins suppress a prompt but never replace fresh TLS possession proof. The
versioned canonical record binds trust to the local certificate, positive nonwrapping
revision, sorted unique peer fingerprints and an exact bounded byte length. Default
capacity is 64 peers, configurable through the contract to 1–1024; lowering capacity
below saved membership rejects the store rather than evicting users silently.

One serialized command owner must govern trust and associated pairing states. Revoke
invalidates live pairing before storage; failed publication or revision exhaustion
poisons reconnect admission. The multi-link broker must revoke every matching live
pairing before allowing reconnect. No policy performs UI/native lifecycle operations.

Linux and Android adapters hold stable exclusive locks, private bounded files and
explicit transaction markers. Marker persistence precedes record publication; complete
replacement and directory synchronization precede marker retirement and its final
sync. Pending or temporary state rejects startup without automatic repair. A failure
before the intent marker becomes durable cannot promise durable revocation; the caller
receives failure and live admission stays closed. No successful revoke is reported
until all required operations complete. Linux identity uses shared private-directory
primitives with all T867 tests retained. Android uses checked API26+ NIO operations,
not AtomicFile's best-effort completion helper.

Validation, offline:
- 229 common Rust tests, including eight T868 tests, and 28 host stock-TLS/storage
  tests, including six T868 tests, pass. Eight native fault/process-exit boundaries
  preserve either revoked state or explicit interrupted-state rejection.
- 14 Android tests pass, including shared golden/invalid vectors, 4096 bounded
  header mutations, full capacity, stale approval and failed persistence cases.
- Normal gate passes all 35 scoped Rust functions (minimum 93.33%) and all 27
  inventoried Kotlin functions (100%). Supplemented native JaCoCo counters cover
  35 maintained Android methods/constructors/getters, all 100%, pending T848.
- Complexity passes all scoped production/test methods; maximum eight.
- Actual tablet Android16/API36 filesystem probe passes remember, restart, revoke,
  exclusive ownership and pending/temp rejection, then removes its disposable state.
  APK was copied and run through app_process under the existing profile app UID;
  nothing was installed or restarted, and no existing identity/trust files were read.

The native smoke probe does not prove power-loss durability, TLS pairing composition
or media integration. T852 retains persisted stock-TLS composition; T853–T857 retain
production broker/service/UI/display. Older Android qualification remains T863 and
Windows has no native store adapter claim. No downloads occurred.
